use core::fmt;
use core::marker::PhantomData;
use core::mem;
use core::ptr;

use crate::record::AssocRecord;
use crate::traits::{Reclaim, Retire};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Typed
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct Typed<T, H>(PhantomData<(T, H)>);

/********** impl Clone ****************************************************************************/

impl<T, H> Clone for Typed<T, H> {
    #[inline]
    fn clone(&self) -> Self {
        Typed(PhantomData)
    }
}

/********** impl Copy *****************************************************************************/

impl<T, H> Copy for Typed<T, H> {}

/********** impl Debug ****************************************************************************/

impl<T, H> fmt::Debug for Typed<T, H> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Typed").finish()
    }
}

/********** impl Default **************************************************************************/

impl<T, H> Default for Typed<T, H> {
    #[inline]
    fn default() -> Self {
        Typed(PhantomData)
    }
}

/********** impl Retire ***************************************************************************/

unsafe impl<T, H> Retire<T> for Typed<T, H> {
    #[inline]
    unsafe fn retire(ptr: *mut T) -> *mut Self::Retired {
        // no need for conversion, ptr is retired (and reclaimed) as is, i.e., with all type
        // information statically known
        ptr
    }
}

/********** impl Reclaim **************************************************************************/

unsafe impl<T, H> Reclaim for Typed<T, H> {
    /// There is no additional drop context required since all type information
    /// is statically known.
    type Header = H;
    type Retired = T;

    #[inline]
    unsafe fn reclaim(retired: *mut Self::Retired) {
        // use the offset to calculate the pointer from the data to the record containing it and
        // drop the entire record.
        let record = AssocRecord::<T, Self>::ptr_from_data(retired);
        mem::drop(Box::from_raw(record));
    }

    /*#[inline]
    unsafe fn convert_to_data(retired: *mut Self::Reclaimable) -> *mut () {
        retired.cast()
    }

    #[inline]
    unsafe fn convert_to_header(retired: *mut Self::Reclaimable) -> *mut Self::Header {
        AssocRecord::<T, Self>::header_from_data(retired.cast())
    }*/
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Erased
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct Erased<H>(PhantomData<H>);

/********** impl Clone ****************************************************************************/

impl<H> Clone for Erased<H> {
    #[inline]
    fn clone(&self) -> Self {
        Erased(PhantomData)
    }
}

/********** impl Copy *****************************************************************************/

impl<H> Copy for Erased<H> {}

/********** impl Retire ***************************************************************************/

unsafe impl<T, H> Retire<T> for Erased<H> {
    #[inline]
    unsafe fn retire(ptr: *mut T) -> *mut Self::Retired {
        let record = AssocRecord::<T, Self>::ptr_from_data(ptr);
        // set the record's drop context
        (*record).header.drop = |retired| {
            // restore the precise type of the record
            let record: *mut AssocRecord<T, Self> = retired.cast();
            // drop the entire record
            mem::drop(Box::from_raw(record));
        };
        (*record).header.data = ptr.cast();

        // return a type erased pointer to the record with its correctly set
        // drop context
        record.cast()
    }
}

/********** impl Reclaim **************************************************************************/

unsafe impl<H> Reclaim for Erased<H> {
    type Header = DynHeader<H>;
    type Retired = ();

    #[inline]
    unsafe fn reclaim(retired: *mut Self::Retired) {
        // retired is a type-erased pointer to some `AssocRecord<_, R>`, which stores the drop
        // context as its first field, since records are #[repr(C)]
        let header: *mut DynHeader<H> = retired.cast();
        let drop = (*header).drop;

        drop(retired.cast());
    }

    /*#[inline]
    unsafe fn convert_to_data(retired: *mut Self::Reclaimable) -> *mut () {
        let drop_ctx: *mut DropCtx = retired.cast();
        (*drop_ctx).data
    }

    #[inline]
    unsafe fn convert_to_header(retired: *mut Self::Reclaimable) -> *mut Self::Header {
        let record: *mut AssocRecord<(), Self> = retired.cast();
        &(*record).header as *const _ as *mut _
    }*/
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Leaking
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Copy, Clone, Debug, Default, Ord, PartialOrd, Eq, PartialEq)]
pub struct Leaking;

/********** impl Retire ***************************************************************************/

unsafe impl<T> Retire<T> for Leaking {
    #[inline(always)]
    unsafe fn retire(ptr: *mut T) -> *mut Self::Retired {
        ptr.cast()
    }
}

/********** impl Reclaim **************************************************************************/

unsafe impl Reclaim for Leaking {
    type Header = ();
    type Retired = ();

    #[inline(always)]
    unsafe fn reclaim(_: *mut Self::Retired) {}

    /*#[inline(always)]
    unsafe fn convert_to_data(retired: *mut Self::Reclaimable) -> *mut () {
        retired.cast()
    }

    #[inline(always)]
    unsafe fn convert_to_header(retired: *mut Self::Reclaimable) -> *mut Self::Header {
        retired.cast()
    }*/
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// DynHeader
////////////////////////////////////////////////////////////////////////////////////////////////////

#[repr(C)]
pub struct DynHeader<H> {
    /// Function pointer to the type-erased drop function.
    drop: unsafe fn(*mut ()),
    /// Pointer to the records's data.
    pub(crate) data: *mut (),
    /// The header itself.
    pub header: H,
}

/********** impl inherent *************************************************************************/

impl<H> DynHeader<H> {
    #[inline]
    pub fn new(header: H) -> Self {
        Self { drop: |_| {}, data: ptr::null_mut(), header }
    }
}

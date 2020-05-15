use core::fmt;
use core::marker::PhantomData;
use core::mem;
use core::ptr;

use crate::record::AssocRecord;
use crate::traits::{Reclaim, Retire, Sealed};

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

impl<T, H: Default> Retire<T> for Typed<T, H> {
    #[inline]
    unsafe fn retire(ptr: *mut T) -> *mut Self::Reclaimable {
        // no need for conversion, ptr is retired (and reclaimed) as is, i.e., with all type
        // information statically known
        ptr
    }
}

/********** impl Sealed ***************************************************************************/

impl<T, H: Default> Sealed for Typed<T, H> {}

/********** impl Reclaim **************************************************************************/

impl<T, H: Default> Reclaim for Typed<T, H> {
    /// There is no additional drop context required since all type information
    /// is statically known.
    type DropCtx = ();
    type Header = H;
    type Reclaimable = T;

    const VIRTUAL_DROP: bool = false;

    #[inline]
    unsafe fn reclaim(retired: *mut Self::Reclaimable) {
        // use the offset to calculate the pointer from the data to the record containing it and
        // drop the entire record.
        let record = AssocRecord::<T, Self>::ptr_from_data(retired);
        mem::drop(Box::from_raw(record));
    }

    #[inline]
    unsafe fn convert_to_data(retired: *mut Self::Reclaimable) -> *mut () {
        retired.cast()
    }

    #[inline]
    unsafe fn convert_to_header(retired: *mut Self::Reclaimable) -> *mut Self::Header {
        AssocRecord::<T, Self>::header_from_data(retired.cast())
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Erased
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Copy, Clone)]
pub struct Erased<H>(PhantomData<H>);
/*
/********** impl Clone ****************************************************************************/

impl<H> Clone for Erased<H> {
    #[inline]
    fn clone(&self) -> Self {
        Erased(PhantomData)
    }
}

/********** impl Copy *****************************************************************************/

impl<H> Copy for Erased<H> {}*/

/********** impl Retire ***************************************************************************/

impl<T, H: Default> Retire<T> for Erased<H> {
    #[inline]
    unsafe fn retire(ptr: *mut T) -> *mut Self::Reclaimable {
        let record = AssocRecord::<T, Self>::ptr_from_data(ptr);
        // set the record's drop context
        (*record).drop_ctx = DropCtx {
            // drop receives a type erased pointer that must be the same as the pointer returned
            // from this function
            drop: |retired| {
                // restore the precise type of the record
                let record: *mut AssocRecord<T, Self> = retired.cast();
                // drop the entire record
                mem::drop(Box::from_raw(record));
            },
            data: ptr.cast(),
        };

        // return a type erased pointer to the record with its correctly set
        // drop context
        record.cast()
    }
}

/********** impl Sealed ***************************************************************************/

impl<H: Default> Sealed for Erased<H> {}

/********** impl Reclaim **************************************************************************/

impl<H: Default> Reclaim for Erased<H> {
    type DropCtx = DropCtx;
    type Header = H;
    type Reclaimable = ();

    const VIRTUAL_DROP: bool = true;

    #[inline]
    unsafe fn reclaim(retired: *mut Self::Reclaimable) {
        // retired is a type-erased pointer to some `AssocRecord<_, R>`, which stores the drop
        // context as its first field, since records are #[repr(C)]
        let drop_ctx: *mut DropCtx = retired.cast();
        let drop = (*drop_ctx).drop;

        drop(retired.cast());
    }

    #[inline]
    unsafe fn convert_to_data(retired: *mut Self::Reclaimable) -> *mut () {
        let drop_ctx: *mut DropCtx = retired.cast();
        (*drop_ctx).data
    }

    #[inline]
    unsafe fn convert_to_header(retired: *mut Self::Reclaimable) -> *mut Self::Header {
        let record: *mut AssocRecord<(), Self> = retired.cast();
        &(*record).header as *const _ as *mut _
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// DropCtx
////////////////////////////////////////////////////////////////////////////////////////////////////

#[repr(C)]
pub struct DropCtx {
    drop: unsafe fn(*mut ()),
    pub(crate) data: *mut (),
}

/********** impl Default **************************************************************************/

impl Default for DropCtx {
    fn default() -> Self {
        Self { drop: |_| {}, data: ptr::null_mut() }
    }
}

use core::marker::PhantomData;
use core::ptr;

use crate::record::Record;
use crate::traits::{AssocItem, Reclaim, ReclaimStrategy, Retire};

////////////////////////////////////////////////////////////////////////////////////////////////////
// CanRetire<T> (sealed trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub unsafe trait CanRetire<T> {
    type Item: Sized;

    unsafe fn convert(retired: *mut T) -> *mut Self::Item;
}

/********** blanket impl **************************************************************************/

unsafe impl<T, R: Reclaim> Retire<T> for R
where
    R::Strategy: CanRetire<T, Item = AssocItem<R>>,
{
    unsafe fn convert(retired: *mut T) -> *mut AssocItem<R> {
        <R::Strategy as CanRetire<T>>::convert(retired)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Typed
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct Typed<T, R>(PhantomData<(T, R)>);

/********** impl ReclaimStrategy ******************************************************************/

impl<T, R: Reclaim<Strategy = Self>> ReclaimStrategy for Typed<T, R> {
    type DropCtx = ();
    type Item = T;

    #[inline]
    unsafe fn reclaim(retired: *mut Self::Item) {
        let record: *mut Record<T, R> = Record::ptr_from_data(retired);
        Box::from_raw(record);
    }
}

/********** impl CanRetire ************************************************************************/

unsafe impl<T, R: Reclaim<Strategy = Self>> CanRetire<T> for Typed<T, R> {
    type Item = T;

    #[inline]
    unsafe fn convert(retired: *mut T) -> *mut Self::Item {
        retired
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Erased
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct Erased<R>(PhantomData<R>);

/********** impl ReclaimStrategy ******************************************************************/

impl<R: Reclaim<Strategy = Self>> ReclaimStrategy for Erased<R> {
    type DropCtx = DropCtx;
    type Item = ();

    #[inline]
    unsafe fn reclaim(retired: *mut Self::Item) {
        let drop_ctx: *mut DropCtx = retired.cast();
        let drop = (*drop_ctx).drop;
        drop(retired);
    }
}

/********** impl CanRetire ************************************************************************/

unsafe impl<T, R: Reclaim<Strategy = Self>> CanRetire<T> for Erased<R> {
    type Item = ();

    #[inline]
    unsafe fn convert(retired: *mut T) -> *mut Self::Item {
        // calculate the pointer to the record using the known memory offsets.
        let record: *mut Record<T, R> = Record::ptr_from_data(retired);
        // set the drop context required for reclaiming
        (*record).drop_ctx = DropCtx {
            drop: |ptr| {
                let record: *mut Record<T, R> = ptr.cast();
                Box::from_raw(record);
            },
            data: retired.cast(),
        };

        record.cast()
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
    #[inline]
    fn default() -> Self {
        Self { drop: |_| {}, data: ptr::null_mut() }
    }
}

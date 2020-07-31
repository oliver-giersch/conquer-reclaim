#[cfg(feature = "nightly")]
use core::marker::Unsize;

use crate::traits::{Reclaim, ReclaimBase};

////////////////////////////////////////////////////////////////////////////////////////////////////
// TypedReclaim
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct TypedReclaim<R>(R);

/********** impl Reclaim **************************************************************************/

unsafe impl<T, R: ReclaimBase<Retired = T>> Reclaim<T> for TypedReclaim<R> {
    type Base = R;

    #[inline(always)]
    unsafe fn retire(ptr: *mut T) -> *mut T {
        ptr
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ErasedReclaim
////////////////////////////////////////////////////////////////////////////////////////////////////

#[cfg(feature = "nightly")]
pub struct ErasedReclaim<R>(R);

/********** impl Reclaim **************************************************************************/

#[cfg(feature = "nightly")]
unsafe impl<T, R: ReclaimBase> Reclaim<T> for ErasedReclaim<R>
where
    T: Unsize<R::Retired>,
{
    type Base = R;

    #[inline(always)]
    unsafe fn retire(ptr: *mut T) -> *mut R::Retired {
        ptr as _
    }
}

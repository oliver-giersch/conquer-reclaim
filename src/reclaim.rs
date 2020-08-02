use core::any::Any;

use crate::traits::{Reclaim, ReclaimBase};

////////////////////////////////////////////////////////////////////////////////////////////////////
// TypedReclaim
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct TypedReclaim<R>(pub R);

/********** impl ReclaimBase **********************************************************************/

unsafe impl<R> ReclaimBase for TypedReclaim<R>
where
    R: ReclaimBase,
{
    type Header = R::Header;
    type Retired = R::Retired;
}

/********** impl Reclaim **************************************************************************/

unsafe impl<T, R> Reclaim<T> for TypedReclaim<R>
where
    R: ReclaimBase<Retired = T>,
{
    #[inline(always)]
    unsafe fn retire(ptr: *mut T) -> *mut T {
        ptr
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ErasedReclaim
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct ErasedReclaim<R>(pub R);

/********** impl ReclaimBase **********************************************************************/

unsafe impl<R> ReclaimBase for ErasedReclaim<R>
where
    R: ReclaimBase<Retired = dyn Any>,
{
    type Header = R::Header;
    type Retired = dyn Any;
}

/********** impl Reclaim **************************************************************************/

unsafe impl<T: 'static, R> Reclaim<T> for ErasedReclaim<R>
where
    R: ReclaimBase<Retired = dyn Any>,
{
    #[inline(always)]
    unsafe fn retire(ptr: *mut T) -> *mut dyn Any {
        ptr as _
    }
}

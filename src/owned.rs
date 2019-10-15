use core::mem;
use core::ptr::NonNull;

use conquer_pointer::NonNullable;
use typenum::Unsigned;

use crate::atomic::Atomic;
use crate::internal::Internal;
use crate::record::Record;
use crate::traits::{Reclaim, ReclaimPointer};
use crate::Owned;

/********** impl ReclaimPointer *******************************************************************/

impl<T, R: Reclaim, N: Unsigned> ReclaimPointer for Owned<T, R, N> {
    impl_reclaim_pointer!();
}

/********** impl Drop *****************************************************************************/

impl<T, R: Reclaim, N: Unsigned> Drop for Owned<T, R, N> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            let record = Record::<_, R>::from_raw(self.inner.decompose_ptr());
            mem::drop(Box::from_raw(record.as_ptr()));
        }
    }
}

/********** impl From *****************************************************************************/

impl<T, R: Reclaim, N: Unsigned> From<T> for Owned<T, R, N> {
    #[inline]
    fn from(owned: T) -> Self {
        unimplemented!()
    }
}

/********** impl NonNullable **********************************************************************/

impl<T, R: Reclaim, N: Unsigned> NonNullable for Owned<T, R, N> {
    type Item = T;

    #[inline]
    fn as_const_ptr(&self) -> *const Self::Item {
        self.inner.decompose_ptr() as *const _
    }

    #[inline]
    fn as_mut_ptr(&self) -> *mut Self::Item {
        self.inner.decompose_ptr()
    }

    #[inline]
    fn as_non_null(&self) -> NonNull<Self::Item> {
        self.inner.decompose_non_null()
    }
}

/********** impl Internal *************************************************************************/

impl<T, R, N> Internal for Owned<T, R, N> {}

use core::ptr::NonNull;

use conquer_pointer::NonNullable;
use typenum::Unsigned;

use crate::retired::Retired;
use crate::traits::{Reclaim, ReclaimHandle};
use crate::Unlinked;

/********** impl inherent *************************************************************************/

impl<T, R: Reclaim, N: Unsigned> Unlinked<T, R, N> {
    /// TODO: Docs...
    #[inline]
    pub unsafe fn retire(self, handle: &impl ReclaimHandle<Reclaimer = R>)
    where
        T: 'static,
    {
        self.retire_unchecked(handle)
    }

    /// TODO: Docs...
    #[inline]
    pub unsafe fn retire_unchecked(self, handle: &impl ReclaimHandle<Reclaimer = R>) {
        let retired: Retired<R> = Retired::new_unchecked(self.inner.decompose_non_null());
        handle.retire(retired);
    }
}

/********** impl NonNullable **********************************************************************/

impl<T, R, N: Unsigned> NonNullable for Unlinked<T, R, N> {
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

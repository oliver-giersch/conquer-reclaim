use core::marker::PhantomData;

use conquer_pointer::{MarkedNonNull, MarkedNonNullable, MarkedPtr, NonNullable};
use typenum::Unsigned;

use crate::retired::Retired;
use crate::traits::{Reclaimer, ReclaimerHandle};
use crate::Unlinked;

/********** impl inherent *************************************************************************/

impl<T, R: Reclaimer, N: Unsigned> Unlinked<T, R, N> {
    #[inline]
    pub unsafe fn from_marked_ptr(ptr: MarkedPtr<T, N>) -> Self {
        Self { inner: MarkedNonNull::new_unchecked(ptr), _marker: PhantomData }
    }

    #[inline]
    pub unsafe fn from_marked_non_null(ptr: MarkedNonNull<T, N>) -> Self {
        Self { inner: ptr, _marker: PhantomData }
    }

    #[inline]
    pub fn as_marked_ptr(&self) -> MarkedPtr<T, N> {
        self.inner.into_marked_ptr()
    }

    #[inline]
    pub fn into_marked_ptr(self) -> MarkedPtr<T, N> {
        self.inner.into_marked_ptr()
    }

    #[inline]
    pub fn as_marked_non_null(&self) -> MarkedNonNull<T, N> {
        self.inner
    }

    #[inline]
    pub fn into_marked_non_null(self) -> MarkedNonNull<T, N> {
        self.inner
    }

    #[inline]
    pub fn clear_tag(self) -> Self {
        unimplemented!()
    }

    #[inline]
    pub fn set_tag(self, tag: usize) -> Self {
        unimplemented!()
    }

    /// TODO: Docs...
    #[inline]
    pub unsafe fn retire(self, handle: impl ReclaimerHandle<Reclaimer = R>)
    where
        T: 'static,
    {
        self.retire_unchecked(handle)
    }

    /// TODO: Docs...
    #[inline]
    pub unsafe fn retire_unchecked(self, handle: impl ReclaimerHandle<Reclaimer = R>) {
        let retired: Retired<R> = Retired::new_unchecked(self.inner.decompose_non_null());
        handle.retire(retired);
    }
}

/********** impl MarkedNonNullable ****************************************************************/

impl<T, R, N: Unsigned> MarkedNonNullable for Unlinked<T, R, N> {
    impl_marked_non_nullable!();
}

/********** impl NonNullable **********************************************************************/

impl<T, R, N: Unsigned> NonNullable for Unlinked<T, R, N> {
    impl_non_nullable!();
}

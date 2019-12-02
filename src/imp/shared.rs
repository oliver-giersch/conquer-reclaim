use core::fmt;
use core::marker::PhantomData;
use core::ptr::NonNull;

use conquer_pointer::{MarkedNonNull, MarkedNonNullable, MarkedPtr, NonNullable};
use typenum::Unsigned;

use crate::internal::Internal;
use crate::traits::Reclaimer;
use crate::Shared;

/********** impl Clone ****************************************************************************/

impl<T, R, N> Clone for Shared<'_, T, R, N> {
    #[inline]
    fn clone(&self) -> Self {
        Self { inner: self.inner, _marker: PhantomData }
    }
}

/********** impl Copy *****************************************************************************/

impl<T, R, N> Copy for Shared<'_, T, R, N> {}

/********** impl inherent *************************************************************************/

impl<T, R, N> Shared<'_, T, R, N> {
    #[inline]
    pub const fn null() -> Self {
        Self { inner: MarkedPtr::null(), _marker: PhantomData }
    }
}

impl<'g, T, R: Reclaimer, N: Unsigned> Shared<'g, T, R, N> {
    #[inline]
    pub unsafe fn from_marked_ptr(ptr: MarkedPtr<T, N>) -> Self {
        Self { inner: ptr, _marker: PhantomData }
    }

    #[inline]
    pub fn is_null(self) -> bool {
        self.inner.is_null()
    }

    #[inline]
    pub fn into_marked_ptr(self) -> MarkedPtr<T, N> {
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

    #[inline]
    pub fn decompose(self) -> (Self, usize) {
        unimplemented!()
    }

    #[inline]
    pub fn decompose_tag(self) -> usize {
        self.inner.decompose_tag()
    }

    #[inline]
    pub unsafe fn decompose_ref(self) -> (Option<&'g T>, usize) {
        self.inner.decompose_ref()
    }

    #[inline]
    pub unsafe fn as_ref(self) -> Option<&'g T> {
        self.inner.as_ref()
    }

    #[inline]
    pub unsafe fn deref(self) -> &'g T {
        &*self.inner.decompose_ptr()
    }

    #[inline]
    pub unsafe fn cast<'a, U>(self) -> Shared<'a, U, R, N> {
        unimplemented!()
    }
}

/********** impl Debug ****************************************************************************/

impl<T: fmt::Debug, R, N: Unsigned> fmt::Debug for Shared<'_, T, R, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Shared")
            .field("ref", self.as_ref())
            .field("tag", &self.inner.decompose_tag())
            .finish()
    }
}

/********** impl Pointer **************************************************************************/

impl<T, R: Reclaimer, N: Unsigned> fmt::Pointer for Shared<'_, T, R, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.inner.decompose_ptr(), f)
    }
}

/********** impl NonNullable **********************************************************************/

impl<T, R, N: Unsigned> NonNullable for Shared<'_, T, R, N> {
    impl_non_nullable!();
}

/********** impl Internal *************************************************************************/

impl<T, R, N: Unsigned> Internal for Shared<'_, T, R, N> {}

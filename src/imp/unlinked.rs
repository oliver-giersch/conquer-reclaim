use core::marker::PhantomData;

use conquer_pointer::{MarkedNonNull, MarkedNonNullable, MarkedPtr, NonNullable};

use crate::retired::Retired;
use crate::traits::{GlobalReclaimer, Reclaimer};
use crate::typenum::Unsigned;
use crate::Unlinked;

/********** impl inherent *************************************************************************/

impl<T, R: Reclaimer, N: Unsigned> Unlinked<T, R, N> {
    impl_common_from!();
    impl_common!();

    #[inline]
    pub fn into_retired(self) -> Retired<R> {
        Retired::new(self.inner.decompose_non_null())
    }

    #[inline]
    pub unsafe fn deref(&self) -> &T {
        self.inner.as_ref()
    }

    #[inline]
    pub unsafe fn decompose_ref(&self) -> (&T, usize) {
        self.inner.decompose_ref()
    }

    #[inline]
    pub unsafe fn cast<U>(self) -> Unlinked<U, R, N> {
        Unlinked { inner: self.inner.cast(), _marker: PhantomData }
    }
}

impl<T, R: GlobalReclaimer, N: Unsigned> Unlinked<T, R, N> {
    #[inline]
    pub unsafe fn retire(self)
    where
        T: 'static,
    {
        self.retire_unchecked()
    }

    #[inline]
    pub unsafe fn retire_unchecked(self) {
        R::retire(self.into_retired())
    }
}

// TODO: Debug, Pointer, ..

/********** impl MarkedNonNullable ****************************************************************/

impl<T, R: Reclaimer, N: Unsigned> MarkedNonNullable for Unlinked<T, R, N> {
    impl_marked_non_nullable!();
}

/********** impl NonNullable **********************************************************************/

impl<T, R, N: Unsigned> NonNullable for Unlinked<T, R, N> {
    impl_non_nullable!();
}

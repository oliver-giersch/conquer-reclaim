use core::fmt;
use core::marker::PhantomData;

use conquer_pointer::typenum::Unsigned;
use conquer_pointer::{MarkedNonNull, MarkedPtr, Null};

use crate::traits::Reclaim;
use crate::{Maybe, Protected, Shared};

/********** impl Clone ****************************************************************************/

impl<T, R, N> Clone for Protected<'_, T, R, N> {
    #[inline]
    fn clone(&self) -> Self {
        Self { inner: self.inner, _marker: PhantomData }
    }
}

/********** impl Copy *****************************************************************************/

impl<T, R, N> Copy for Protected<'_, T, R, N> {}

/********** impl inherent (const) *****************************************************************/

impl<T, R, N> Protected<'_, T, R, N> {
    /// Creates a new `null` pointer.
    #[inline]
    pub const fn null() -> Self {
        Self { inner: MarkedPtr::null(), _marker: PhantomData }
    }

    #[inline]
    pub const unsafe fn cast<'a, U>(self) -> Protected<'a, U, R, N> {
        Protected { inner: self.inner.cast(), _marker: PhantomData }
    }
}

/********** impl inherent *************************************************************************/

impl<'g, T, R: Reclaim, N: Unsigned> Protected<'g, T, R, N> {
    impl_from_ptr_for_nullable!();
    impl_from_non_null!();

    #[inline]
    pub fn is_null(self) -> bool {
        self.inner.is_null()
    }

    impl_common!();

    #[inline]
    pub fn shared(self) -> Maybe<Shared<'g, T, R, N>> {
        match MarkedNonNull::new(self.inner) {
            Ok(inner) => Maybe::Some(Shared { inner, _marker: PhantomData }),
            Err(Null(tag)) => Maybe::Null(tag),
        }
    }

    #[inline]
    pub unsafe fn shared_unchecked(self) -> Shared<'g, T, R, N> {
        Shared { inner: MarkedNonNull::new_unchecked(self.inner), _marker: PhantomData }
    }

    #[inline]
    pub unsafe fn as_ref(self) -> Option<&'g T> {
        self.inner.as_ref()
    }

    #[inline]
    pub unsafe fn decompose_ref(self) -> (Option<&'g T>, usize) {
        self.inner.decompose_ref()
    }

    #[inline]
    pub unsafe fn deref(self) -> &'g T {
        &*self.inner.decompose_ptr()
    }
}

/********** impl Debug ****************************************************************************/

impl<T, R, N: Unsigned> fmt::Debug for Protected<'_, T, R, N> {
    impl_fmt_debug!(Protected);
}

/********** impl Default **************************************************************************/

impl<T, R, N> Default for Protected<'_, T, R, N> {
    #[inline]
    fn default() -> Self {
        Self::null()
    }
}

/********** impl Pointer **************************************************************************/

impl<T, R, N: Unsigned> fmt::Pointer for Protected<'_, T, R, N> {
    impl_fmt_pointer!();
}

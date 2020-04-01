use core::fmt;
use core::marker::PhantomData;

use conquer_pointer::typenum::Unsigned;
use conquer_pointer::{MarkedNonNull, MarkedPtr};

use crate::atomic::Storable;
use crate::traits::Reclaim;
use crate::Unprotected;

/********** impl Clone ****************************************************************************/

impl<T, R, N> Clone for Unprotected<T, R, N> {
    #[inline]
    fn clone(&self) -> Self {
        Self { inner: self.inner, _marker: PhantomData }
    }
}

/********** impl Copy *****************************************************************************/

impl<T, R, N> Copy for Unprotected<T, R, N> {}

/********** impl inherent (const) *****************************************************************/

impl<T, R, N> Unprotected<T, R, N> {
    #[inline]
    pub const fn null() -> Self {
        Self { inner: MarkedPtr::null(), _marker: PhantomData }
    }

    #[inline]
    pub const fn cast<U>(self) -> Unprotected<U, R, N> {
        Unprotected { inner: self.inner.cast(), _marker: PhantomData }
    }
}

/********** impl inherent *************************************************************************/

impl<T, R, N: Unsigned> Unprotected<T, R, N> {
    impl_from_ptr_for_nullable!();
    impl_from_non_null!();

    #[inline]
    pub fn is_null(self) -> bool {
        self.inner.is_null()
    }

    impl_common!();

    #[inline]
    pub unsafe fn assume_storable(self) -> Storable<T, R, N> {
        Storable::new(self.inner)
    }
}

/********** impl Default **************************************************************************/

impl<T, R, N> Default for Unprotected<T, R, N> {
    #[inline]
    fn default() -> Self {
        Self::null()
    }
}

/********** impl Debug ****************************************************************************/

impl<T, R: Reclaim, N: Unsigned> fmt::Debug for Unprotected<T, R, N> {
    impl_fmt_debug!(Unprotected);
}

/********** impl Pointer **************************************************************************/

impl<T, R: Reclaim, N: Unsigned> fmt::Pointer for Unprotected<T, R, N> {
    impl_fmt_pointer!();
}

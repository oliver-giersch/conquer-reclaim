use core::fmt;
use core::marker::PhantomData;

use conquer_pointer::{MarkedNonNull, MarkedPtr, Null};

use crate::{Maybe, Protected, Shared, Unlinked, Unprotected};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Comparable
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A raw, nullable and potentially marked pointer type this is associated to a
/// [`Reclaimer`][crate::Reclaim] and can be used as the compare argument for
/// a *compare-and-swap* operation.
pub struct Comparable<T, R, const N: usize> {
    inner: MarkedPtr<T, N>,
    _marker: PhantomData<R>,
}

/********** impl Clone ****************************************************************************/

impl<T, R, const N: usize> Clone for Comparable<T, R, N> {
    #[inline]
    fn clone(&self) -> Self {
        Self { inner: self.inner, _marker: PhantomData }
    }
}

/********** impl Copy *****************************************************************************/

impl<T, R, const N: usize> Copy for Comparable<T, R, N> {}

/********** impl inherent *************************************************************************/

impl<T, R, const N: usize> Comparable<T, R, N> {
    /// Creates a new `null` pointer.
    #[inline]
    pub const fn null() -> Self {
        Self::new(MarkedPtr::null())
    }

    /// Returns the inner raw [`MarkedPtr`].
    #[inline]
    pub const fn into_marked_ptr(self) -> MarkedPtr<T, N> {
        self.inner
    }

    /// Creates a new `Comparable`.
    #[inline]
    pub(crate) const fn new(inner: MarkedPtr<T, N>) -> Self {
        Self { inner, _marker: PhantomData }
    }
}

/********** impl Debug ****************************************************************************/

impl<T, R, const N: usize> fmt::Debug for Comparable<T, R, N> {
    impl_fmt_debug!(Comparable);
}

/********** impl From (Protected) *****************************************************************/

impl<T, R, const N: usize> From<Protected<'_, T, R, N>> for Comparable<T, R, N> {
    #[inline]
    fn from(protected: Protected<'_, T, R, N>) -> Self {
        Self { inner: protected.inner, _marker: PhantomData }
    }
}

/********** impl From (Shared) ********************************************************************/

impl<T, R, const N: usize> From<Shared<'_, T, R, N>> for Comparable<T, R, N> {
    #[inline]
    fn from(shared: Shared<'_, T, R, N>) -> Self {
        Self { inner: shared.inner.into_marked_ptr(), _marker: PhantomData }
    }
}

/********** impl From (Unprotected) ***************************************************************/

impl<T, R, const N: usize> From<Unprotected<T, R, N>> for Comparable<T, R, N> {
    #[inline]
    fn from(unprotected: Unprotected<T, R, N>) -> Self {
        Self { inner: unprotected.inner, _marker: PhantomData }
    }
}

/********** impl Pointer **************************************************************************/

impl<T, R, const N: usize> fmt::Pointer for Comparable<T, R, N> {
    impl_fmt_pointer!();
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Unlink (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// An internal (not exported) trait for transforming some marked pointer type
/// into an appropriate [`Unlinked`] or `null` variant after a successful
/// *compare-and-swap* operation.
pub trait Unlink {
    /// The [`Unlinked`] type.
    type Unlinked;

    /// Converts `self` into the associated unlinked type.
    unsafe fn into_unlinked(self) -> Self::Unlinked;
}

/********** impl Protected ************************************************************************/

impl<T, R, const N: usize> Unlink for Protected<'_, T, R, N> {
    type Unlinked = Maybe<Unlinked<T, R, N>>;

    #[inline]
    unsafe fn into_unlinked(self) -> Self::Unlinked {
        Unprotected { inner: self.inner, _marker: PhantomData }.into_unlinked()
    }
}

/********** impl Shared ***************************************************************************/

impl<T, R, const N: usize> Unlink for Shared<'_, T, R, N> {
    type Unlinked = Unlinked<T, R, N>;

    #[inline]
    unsafe fn into_unlinked(self) -> Self::Unlinked {
        Unlinked { inner: self.inner, _marker: PhantomData }
    }
}

/********** impl Unprotected **********************************************************************/

impl<T, R, const N: usize> Unlink for Unprotected<T, R, N> {
    type Unlinked = Maybe<Unlinked<T, R, N>>;

    #[inline]
    unsafe fn into_unlinked(self) -> Self::Unlinked {
        match MarkedNonNull::new(self.inner) {
            Ok(inner) => Maybe::Some(Unlinked { inner, _marker: PhantomData }),
            Err(Null(tag)) => Maybe::Null(tag),
        }
    }
}

use core::fmt;
use core::marker::PhantomData;
use core::mem;

use conquer_pointer::typenum::Unsigned;
use conquer_pointer::MarkedPtr;

use crate::traits::Reclaim;
use crate::{Owned, Shared};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Storable
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct Storable<T, R, N> {
    inner: MarkedPtr<T, N>,
    _marker: PhantomData<R>,
}

/********** impl Clone ****************************************************************************/

impl<T, R, N> Clone for Storable<T, R, N> {
    #[inline]
    fn clone(&self) -> Self {
        Self { inner: self.inner, _marker: PhantomData }
    }
}

/********** impl Copy *****************************************************************************/

impl<T, R, N> Copy for Storable<T, R, N> {}

/********** impl inherent *************************************************************************/

impl<T, R, N> Storable<T, R, N> {
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

    /// Creates a new `Storable`.
    #[inline]
    pub(crate) const fn new(inner: MarkedPtr<T, N>) -> Self {
        Self { inner, _marker: PhantomData }
    }
}

/********** impl Debug ****************************************************************************/

impl<T, R, N: Unsigned> fmt::Debug for Storable<T, R, N> {
    impl_fmt_debug!(Storable);
}

/********** impl From (Owned) *********************************************************************/

impl<T, R: Reclaim<T>, N: Unsigned> From<Owned<T, R, N>> for Storable<T, R, N> {
    #[inline]
    fn from(owned: Owned<T, R, N>) -> Self {
        let storable = Self { inner: owned.inner.into(), _marker: PhantomData };
        mem::forget(owned);
        storable
    }
}

/********** impl From (Shared) ********************************************************************/

impl<T, R, N> From<Shared<'_, T, R, N>> for Storable<T, R, N> {
    #[inline]
    fn from(shared: Shared<'_, T, R, N>) -> Self {
        Self { inner: shared.inner.into_marked_ptr(), _marker: PhantomData }
    }
}

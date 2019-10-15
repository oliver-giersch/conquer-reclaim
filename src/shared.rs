use core::borrow::Borrow;
use core::convert::AsRef;
use core::fmt;
use core::marker::PhantomData;
use core::ptr::NonNull;

use conquer_pointer::NonNullable;
use typenum::Unsigned;

use crate::internal::Internal;
use crate::traits::{Reclaim, ReclaimPointer};
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

/********** impl ReclaimPointer *******************************************************************/

impl<T, R: Reclaim, N: Unsigned> ReclaimPointer for Shared<'_, T, R, N> {
    impl_reclaim_pointer!();
}

/********** impl inherent *************************************************************************/

impl<'g, T, R: Reclaim, N: Unsigned> Shared<'g, T, R, N> {
    // impl_inherent!(shared)

    /// Consumes and decomposes the [`Shared`] reference, returning only the
    /// reference itself.
    ///
    /// # Example
    ///
    /// Derefencing a [`Shared`] ties the returned reference to the (shorter)
    /// lifetime of the `shared` itself.
    /// Use this function to get a reference with the full lifetime `'g`.
    ///
    /// ```
    /// use core::sync::atomic::Ordering::Relaxed;
    ///
    /// use reclaim::prelude::*;
    /// use reclaim::typenum::U0;
    /// use reclaim::leak::Shared;
    ///
    /// type Atomic<T> = reclaim::leak::Atomic<T, U0>;
    /// type Guard = reclaim::leak::Guard;
    ///
    /// let atomic = Atomic::new("string");
    ///
    /// let mut guard = Guard::new();
    /// let shared = atomic.load(Relaxed, &mut guard);
    ///
    /// let reference = shared.unwrap().into_ref();
    /// assert_eq!(reference, &"string");
    /// ```
    #[inline]
    pub fn into_ref(self) -> &'g T {
        unsafe { &*self.inner.decompose_ptr() }
    }

    /// Decomposes the (marked) [`Shared`] reference, returning the reference
    /// itself and the separated tag.
    #[inline]
    pub fn decompose_ref(self) -> (&'g T, usize) {
        unsafe { self.inner.decompose_ref() }
    }

    /// Casts the [`Shared`] to a reference to a different type and with a different lifetime.
    ///
    /// This can be useful to extend the lifetime of a [`Shared`] in cases the borrow checker is
    /// unable to correctly determine the relationship between mutable borrows of guards and the
    /// resulting shared references.
    ///
    /// # Safety
    ///
    /// The caller has to ensure the cast is valid both in terms of type and lifetime.
    #[inline]
    pub unsafe fn cast<'h, U>(shared: Self) -> Shared<'h, U, R, N> {
        Shared { inner: shared.inner.cast(), _marker: PhantomData }
    }
}

/********** impl AsRef ****************************************************************************/

impl<T, R, N: Unsigned> AsRef<T> for Shared<'_, T, R, N> {
    #[inline]
    fn as_ref(&self) -> &T {
        unsafe { self.inner.as_ref() }
    }
}

/********** impl Borrow ***************************************************************************/

impl<T, R, N: Unsigned> Borrow<T> for Shared<'_, T, R, N> {
    #[inline]
    fn borrow(&self) -> &T {
        self.as_ref()
    }
}

/********** impl Debug ****************************************************************************/

impl<T: fmt::Debug, R, N: Unsigned> fmt::Debug for Shared<'_, T, R, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Shared")
            .field("ref", self.as_ref())
            .field("tag", self.inner.decompose_tag())
            .finish()
    }
}

/********** impl Pointer **************************************************************************/

impl<T, R: Reclaim, N: Unsigned> fmt::Pointer for Shared<'_, T, R, N> {
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

impl<T, R, N> Internal for Shared<'_, T, R, N> {}

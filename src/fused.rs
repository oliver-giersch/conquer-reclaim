//! Wrapper types for *fusing* guard instances and protected pointers or
//! references.

use core::convert::TryInto;
use core::fmt;
use core::mem;

use conquer_pointer::{MarkedNonNull, MarkedPtr, Null};

use crate::{Protect, Protected, Shared};

// *************************************************************************************************
// FusedProtected
// *************************************************************************************************

/// An owned guard fused with a (nullable) [`Protected`] pointer.
pub struct FusedProtected<T, G, const N: usize> {
    /// The owned guard.
    pub(crate) guard: G,
    /// The protected pointer.
    pub(crate) protected: MarkedPtr<T, N>,
}

/********** impl inherent *************************************************************************/

impl<T, G: Protect<T>, const N: usize> FusedProtected<T, G, N> {
    /// Creates a new [`FusedProtected`] from `guard` with a `null` pointer.
    #[inline]
    pub fn null(guard: G) -> Self {
        Self { guard, protected: MarkedPtr::null() }
    }

    /// Returns the inner [`Protected`] pointer.
    #[inline]
    pub fn as_protected(&self) -> Protected<T, G::Reclaim, N> {
        unsafe { Protected::from_marked_ptr(self.protected) }
    }

    /// Attempts to convert `self` into a [`FusedShared`].
    ///
    /// # Errors
    ///
    /// Fails, if `self` holds a `null` pointer, in which case an [`Err`] with
    /// the original value and a [`Null`] instance is returned.
    #[inline]
    pub fn into_fused_shared(self) -> Result<FusedShared<T, G, N>, (Self, Null)> {
        match self.protected.try_into() {
            Ok(shared) => Ok(FusedShared { guard: self.guard, shared }),
            Err(null) => Err((self, null)),
        }
    }

    /// Consumes `self` and returns the contained guard instance, forfeiting its
    /// currently protected value.
    #[inline]
    pub fn into_guard(self) -> G {
        self.guard
    }
}

/********** impl Debug ****************************************************************************/

impl<T, G: Protect<T>, const N: usize> fmt::Debug for FusedProtected<T, G, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FusedProtected {{ ... }}")
    }
}

// *************************************************************************************************
// FusedProtectedRef
// *************************************************************************************************

/// A borrowed guard fused with a (nullable) [`Protected`] pointer.
pub struct FusedProtectedRef<'g, T, G, const N: usize> {
    pub(crate) guard: &'g mut G,
    pub(crate) protected: MarkedPtr<T, N>,
}

/********** impl inherent *************************************************************************/

impl<'g, T, G: Protect<T>, const N: usize> FusedProtectedRef<'g, T, G, N> {
    /// Creates a new [`FusedProtectedRef`] from `guard` with a `null` pointer.
    #[inline]
    pub fn null(guard: &'g mut G) -> Self {
        Self { guard, protected: MarkedPtr::null() }
    }

    /// Returns the inner [`Protected`] pointer.
    #[inline]
    pub fn as_protected(&self) -> Protected<T, G::Reclaim, N> {
        unsafe { Protected::from_marked_ptr(self.protected) }
    }

    /// Attempts to convert `self` into a [`FusedSharedRef`].
    #[inline]
    pub fn into_fused_shared_ref(self) -> Result<FusedSharedRef<'g, T, G, N>, (Self, Null)> {
        match self.protected.try_into() {
            Ok(shared) => Ok(FusedSharedRef { guard: self.guard, shared }),
            Err(null) => Err((self, null)),
        }
    }

    #[inline]
    pub fn into_guard_ref(self) -> &'g mut G {
        self.guard
    }
}

/********** impl Debug ****************************************************************************/

impl<T, G: Protect<T>, const N: usize> fmt::Debug for FusedProtectedRef<'_, T, G, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FusedProtectedRef {{ ... }}")
    }
}

// *************************************************************************************************
// FusedShared
// *************************************************************************************************

/// An owned guard fused with a (non-nullable) [`Shared`] reference.
pub struct FusedShared<T, G, const N: usize> {
    pub(crate) guard: G,
    pub(crate) shared: MarkedNonNull<T, N>,
}

/********** impl inherent *************************************************************************/

impl<T, G: Protect<T>, const N: usize> FusedShared<T, G, N> {
    #[inline]
    pub fn adopt(mut guard: G, mut other: FusedShared<T, G, N>) -> (Self, G) {
        mem::swap(&mut guard, &mut other.guard);
        (Self { guard, shared: other.shared }, other.guard)
    }

    #[inline]
    pub fn as_shared(&self) -> Shared<T, G::Reclaim, N> {
        unsafe { Shared::from_marked_non_null(self.shared) }
    }

    #[inline]
    pub fn transfer_to(mut self, mut guard: G) -> (Self, G) {
        mem::swap(&mut self.guard, &mut guard);
        (Self { guard, shared: self.shared }, self.guard)
    }

    #[inline]
    pub fn transfer_to_ref(mut self, guard: &mut G) -> (FusedSharedRef<T, G, N>, G) {
        mem::swap(&mut self.guard, guard);
        (FusedSharedRef { guard, shared: self.shared }, self.guard)
    }

    #[inline]
    pub fn into_fused_protected(self) -> FusedProtected<T, G, N> {
        FusedProtected { guard: self.guard, protected: self.shared.into_marked_ptr() }
    }

    #[inline]
    pub fn into_guard(self) -> G {
        self.guard
    }
}

/********** impl Debug ****************************************************************************/

impl<T, G: Protect<T>, const N: usize> fmt::Debug for FusedShared<T, G, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FusedShared {{ ... }}")
    }
}

// *************************************************************************************************
// FusedSharedRef
// *************************************************************************************************

pub struct FusedSharedRef<'g, T, G, const N: usize> {
    pub(crate) guard: &'g mut G,
    pub(crate) shared: MarkedNonNull<T, N>,
}

/********** impl inherent *************************************************************************/

impl<'g, T, G: Protect<T>, const N: usize> FusedSharedRef<'g, T, G, N> {
    #[inline]
    pub fn as_shared(&self) -> Shared<T, G::Reclaim, N> {
        unsafe { Shared::from_marked_non_null(self.shared) }
    }

    #[inline]
    pub fn transfer_to(self, mut guard: G) -> FusedShared<T, G, N> {
        mem::swap(self.guard, &mut guard);
        FusedShared { guard, shared: self.shared }
    }

    #[inline]
    pub fn transfer_to_ref<'h>(self, guard: &'h mut G) -> FusedSharedRef<'h, T, G, N> {
        mem::swap(self.guard, guard);
        FusedSharedRef { guard, shared: self.shared }
    }

    #[inline]
    pub fn into_fused_protected_ref(self) -> FusedProtectedRef<'g, T, G, N> {
        FusedProtectedRef { guard: self.guard, protected: self.shared.into_marked_ptr() }
    }

    #[inline]
    pub fn into_shared(self) -> Shared<'g, T, G::Reclaim, N> {
        unsafe { Shared::from_marked_non_null(self.shared) }
    }

    #[inline]
    pub fn into_guard_ref(self) -> &'g mut G {
        self.guard
    }
}

/********** impl Debug ****************************************************************************/

impl<T, G: Protect<T>, const N: usize> fmt::Debug for FusedSharedRef<'_, T, G, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FusedSharedRef {{ ... }}")
    }
}

use core::convert::TryInto;
use core::mem;

use conquer_pointer::{MarkedNonNull, MarkedPtr, Null};

use crate::{Maybe, Protect, Protected, Shared};

////////////////////////////////////////////////////////////////////////////////////////////////////
// FusedProtected
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct FusedProtected<T, G, const N: usize> {
    pub(crate) guard: G,
    pub(crate) protected: MarkedPtr<T, N>,
}

/********** impl inherent *************************************************************************/

impl<T, G: Protect<T>, const N: usize> FusedProtected<T, G, N> {
    #[inline]
    pub fn null(guard: G) -> Self {
        Self { guard, protected: MarkedPtr::null() }
    }

    #[inline]
    pub fn as_protected(&self) -> Protected<T, G::Reclaim, N> {
        unsafe { Protected::from_marked_ptr(self.protected) }
    }

    #[inline]
    pub fn into_fused_shared(self) -> Result<FusedShared<T, G, N>, (Self, Null)> {
        match self.protected.try_into() {
            Ok(shared) => Ok(FusedShared { guard: self.guard, shared }),
            Err(null) => Err((self, null)),
        }
    }

    #[inline]
    pub fn into_guard(self) -> G {
        self.guard
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// FusedShared
////////////////////////////////////////////////////////////////////////////////////////////////////

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
        todo!()
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

////////////////////////////////////////////////////////////////////////////////////////////////////
// FusedProtectedRef
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct FusedProtectedRef<'g, T, G, const N: usize> {
    pub(crate) guard: &'g mut G,
    pub(crate) protected: MarkedPtr<T, N>,
}

/********** impl inherent *************************************************************************/

impl<'g, T, G: Protect<T>, const N: usize> FusedProtectedRef<'g, T, G, N> {
    #[inline]
    pub fn null(guard: &'g mut G) -> Self {
        Self { guard, protected: MarkedPtr::null() }
    }

    #[inline]
    pub fn as_protected(&self) -> Protected<T, G::Reclaim, N> {
        todo!()
    }

    #[inline]
    pub fn adopt(&mut self, mut other: FusedProtected<T, G, N>) -> G {
        mem::swap(self.guard, &mut other.guard);
        self.protected = other.protected;
        other.guard
    }

    #[inline]
    pub fn adopt_ref(&mut self, other: FusedProtectedRef<'_, T, G, N>) {
        mem::swap(self.guard, other.guard);
        self.protected = other.protected;
    }

    #[inline]
    pub fn into_fused_shared_ref(self) -> Result<FusedSharedRef<'g, T, G, N>, (Self, Null)> {
        todo!()
    }

    #[inline]
    pub fn into_guard_ref(self) -> &'g mut G {
        self.guard
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// FusedSharedRef
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct FusedSharedRef<'g, T, G, const N: usize> {
    pub(crate) guard: &'g mut G,
    pub(crate) shared: MarkedNonNull<T, N>,
}

/********** impl inherent *************************************************************************/

impl<'g, T, G: Protect<T>, const N: usize> FusedSharedRef<'g, T, G, N> {
    #[inline]
    pub fn adopt(guard: &'g mut G, mut other: FusedShared<T, G, N>) -> (Self, G) {
        mem::swap(guard, &mut other.guard);
        (Self { guard, shared: other.shared }, other.guard)
    }

    #[inline]
    pub fn as_shared(&self) -> Shared<T, G::Reclaim, N> {
        todo!()
    }

    #[inline]
    pub fn into_shared(self) -> Shared<'g, T, G::Reclaim, N> {
        todo!()
    }

    #[inline]
    pub fn into_guard_ref(self) -> &'g mut G {
        self.guard
    }
}

/********** impl Debug ****************************************************************************/

/*
////////////////////////////////////////////////////////////////////////////////////////////////////
// FusedGuard
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A guard type that is created by *fusing* the guard with the value it should
/// protect.
///
/// See e.g. [`protect_fused`][crate::traits::ProtectExt] for means of creating
/// instances of this type.
pub struct FusedGuard<T, G, const N: usize> {
    pub(crate) guard: G,
    pub(crate) protected: MarkedPtr<T, N>,
}

/********** impl inherent *************************************************************************/

impl<T, G: Protect<T>, const N: usize> FusedGuard<T, G, N> {
    #[inline]
    pub fn as_shared(&self) -> Maybe<Shared<T, G::Reclaim, N>> {
        match self.protected.try_into() {
            Ok(shared) => Maybe::Some(unsafe { Shared::from_marked_non_null(shared) }),
            Err(null) => Maybe::Null(null.tag()),
        }
    }

    #[inline]
    pub fn as_protected(&self) -> Protected<T, G::Reclaim, N> {
        unsafe { Protected::from_marked_ptr(self.protected) }
    }

    #[inline]
    pub fn into_guard(self) -> G {
        self.guard
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// FusedGuardRef
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A guard type that is created by *fusing* a mutable reference to the guard
/// with the value it should protect.
///
/// This allows e.g. moving the guard reference together with the protected
/// dependent value (reference) or [`adopt`][crate::traits::ProtectExt::adopt]ing
/// it from another guard.
///
/// See e.g. [`protect_fused_ref`][crate::traits::ProtectExt::protect_fused_ref]
/// for means of creating 2instances of this type.
pub struct FusedGuardRef<'g, T, G, const N: usize> {
    pub(crate) guard: &'g mut G,
    pub(crate) shared: MarkedNonNull<T, N>,
}

/********** impl inherent *************************************************************************/

impl<'g, T, G: Protect<T>, const N: usize> FusedGuardRef<'g, T, G, N> {
    #[inline]
    pub fn as_shared(&self) -> Shared<T, G::Reclaim, N> {
        unsafe { Shared::from_marked_non_null(self.shared) }
    }

    #[inline]
    pub fn into_shared(self) -> Shared<'g, T, G::Reclaim, N> {
        unsafe { Shared::from_marked_non_null(self.shared) }
    }
}
*/

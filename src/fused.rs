use conquer_pointer::MarkedNonNull;

use crate::{Protect, Shared};

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
    pub(crate) shared: MarkedNonNull<T, N>,
}

/********** impl inherent *************************************************************************/

impl<T, G: Protect<T>, const N: usize> FusedGuard<T, G, N> {
    #[inline]
    pub fn as_shared(&self) -> Shared<T, G::Reclaim, N> {
        unsafe { Shared::from_marked_non_null(self.shared) }
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

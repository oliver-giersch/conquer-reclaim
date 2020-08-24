use conquer_pointer::MarkedNonNull;

use crate::{Protect, Shared};

////////////////////////////////////////////////////////////////////////////////////////////////////
// FusedGuard
////////////////////////////////////////////////////////////////////////////////////////////////////

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

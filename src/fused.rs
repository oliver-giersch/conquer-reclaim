use conquer_pointer::typenum::Unsigned;
use conquer_pointer::MarkedNonNull;

use crate::{Protect, Shared};

////////////////////////////////////////////////////////////////////////////////////////////////////
// FusedGuard
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct FusedGuardRef<'g, T, G, N> {
    pub(crate) guard: &'g mut G,
    pub(crate) shared: MarkedNonNull<T, N>,
}

/********** impl inherent *************************************************************************/

impl<'g, T, G: Protect<T>, N: Unsigned> FusedGuardRef<'g, T, G, N> {
    #[inline]
    pub fn as_shared(&self) -> Shared<T, G::Reclaim, N> {
        unsafe { Shared::from_marked_non_null(self.shared) }
    }

    #[inline]
    pub fn into_shared(self) -> Shared<'g, T, G::Reclaim, N> {
        unsafe { Shared::from_marked_non_null(self.shared) }
    }
}

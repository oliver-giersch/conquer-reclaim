use core::marker::PhantomData;
use core::sync::atomic::Ordering;

use conquer_pointer::{
    MarkedNonNull, MarkedPtr,
    MaybeNull::{NotNull, Null},
};
use typenum::Unsigned;

use crate::atomic::Atomic;
use crate::traits::Protect;
use crate::Shared;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Guarded
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: docs...
pub struct Guarded<T, G, N> {
    guard: G,
    protected: MarkedNonNull<T, N>,
}

/********** impl inherent *************************************************************************/

impl<T, G: Protect, N: Unsigned> Guarded<T, G, N> {
    /// TODO: docs...
    #[inline]
    pub fn try_fuse(
        mut guard: G,
        src: &Atomic<T, G::Reclaimer, N>,
        order: Ordering,
    ) -> Result<Self, G> {
        match guard.protect(src, order) {
            NotNull(shared) => {
                let protected = shared.inner;
                Ok(Self { guard, protected })
            }
            Null(_) => Err(guard),
        }
    }

    /// TODO: docs...
    #[inline]
    pub fn try_fuse_if_equal(
        mut guard: G,
        src: &Atomic<T, G::Reclaimer, N>,
        expected: MarkedPtr<T, N>,
        order: Ordering,
    ) -> Result<Self, G> {
        match guard.protect_if_equal(src, expected, order) {
            Ok(NotNull(shared)) => {
                let protected = shared.inner;
                Ok(Self { guard, protected })
            }
            _ => Err(guard),
        }
    }

    /// Returns a [`Shared`] reference borrowed from the [`Guarded`].
    #[inline]
    pub fn shared(&self) -> Shared<T, G::Reclaimer, N> {
        Shared { inner: self.protected, _marker: PhantomData }
    }

    /// Converts the [`Guarded`] into the internally stored guard.
    ///
    /// If `G` does not implement [`ProtectRegion`], the returned guard is
    /// guaranteed to be [`released`][Protect::release] before being returned.
    #[inline]
    pub fn into_guard(self) -> G {
        let mut guard = self.guard;
        guard.release();
        guard
    }
}

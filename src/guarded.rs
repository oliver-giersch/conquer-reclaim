use core::marker::PhantomData;
use core::sync::atomic::Ordering;

use conquer_pointer::typenum::Unsigned;
use conquer_pointer::{MarkedNonNull, MarkedPtr};

use crate::atomic::Atomic;
use crate::traits::Protect;
use crate::{Maybe, Reclaim, Shared};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Guarded
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: docs...
pub struct Guarded<T, R: Reclaim, N> {
    guard: R::Guard,
    protected: MarkedNonNull<T, N>,
}

/********** impl inherent *************************************************************************/

impl<T, R: Reclaim, N: Unsigned + 'static> Guarded<T, R, N> {
    /// TODO: docs...
    #[inline]
    pub fn try_fuse(
        mut guard: R::Guard,
        atomic: &Atomic<T, R, N>,
        order: Ordering,
    ) -> Result<Self, R::Guard> {
        match guard.protect(atomic, order).shared() {
            Maybe::Some(shared) => {
                let protected = shared.inner;
                Ok(Self { guard, protected })
            }
            Maybe::Null(_) => Err(guard),
        }
    }

    /// TODO: docs...
    #[inline]
    pub fn try_fuse_if_equal(
        mut guard: R::Guard,
        atomic: &Atomic<T, R, N>,
        expected: MarkedPtr<T, N>,
        order: Ordering,
    ) -> Result<Self, R::Guard> {
        match guard.protect_if_equal(atomic, expected, order) {
            Ok(protected) => match protected.shared() {
                Maybe::Some(shared) => {
                    let protected = shared.inner;
                    Ok(Self { guard, protected })
                }
                Maybe::Null(_) => Err(guard),
            },
            Err(_) => Err(guard),
        }
    }

    /// Returns a [`Shared`] reference borrowed from the [`Guarded`].
    #[inline]
    pub fn shared(&self) -> Shared<T, R, N> {
        Shared { inner: self.protected, _marker: PhantomData }
    }

    /// Converts the [`Guarded`] into the internally stored guard.
    ///
    /// If `G` does not implement [`ProtectRegion`], the returned guard is
    /// guaranteed to be [`released`][Protect::release] before being returned.
    #[inline]
    pub fn into_guard(self) -> R::Guard {
        self.guard
    }
}

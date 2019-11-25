use core::sync::atomic::Ordering;

use conquer_pointer::{MarkedOption, MarkedPtr};
use typenum::Unsigned;

use crate::atomic::Atomic;

use crate::traits::{Protect, ProtectRegion, Reclaimer};
use crate::{AcquireResult, Shared};
use std::marker::PhantomData;

////////////////////////////////////////////////////////////////////////////////////////////////////
// GuardRef (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A sealed trait for abstracting over different types for valid guard references.
///
/// For guard types implementing only the [`Protect`](crate::Protect) trait,
/// this trait is only implemented for *mutable* references to this type.
/// For guard types that also implement the
/// [`ProtectRegion`](crate::ProtectRegion) trait, this trait is also
/// implemented for *shared* references.
pub trait GuardRef<'g> {
    /// TODO: Docs...
    type Reclaimer: Reclaimer;

    /// TODO: Docs...
    fn load_protected<T, N: Unsigned>(
        self,
        atomic: &Atomic<T, Self::Reclaimer, N>,
        order: Ordering,
    ) -> MarkedOption<Shared<'g, T, Self::Reclaimer, N>>;

    /// TODO: Docs...
    fn load_protected_if_equal<T, N: Unsigned>(
        self,
        atomic: &Atomic<T, Self::Reclaimer, N>,
        expected: MarkedPtr<T, N>,
        order: Ordering,
    ) -> AcquireResult<'g, T, Self::Reclaimer, N>;
}

/********** impl blanket (Protect) ****************************************************************/

impl<'g, G> GuardRef<'g> for &'g mut G
where
    G: Protect,
{
    type Reclaimer = G::Reclaimer;

    #[inline]
    fn load_protected<T, N: Unsigned>(
        self,
        atomic: &Atomic<T, Self::Reclaimer, N>,
        order: Ordering,
    ) -> MarkedOption<Shared<'g, T, Self::Reclaimer, N>> {
        self.protect(atomic, order)
    }

    #[inline]
    fn load_protected_if_equal<T, N: Unsigned>(
        self,
        atomic: &Atomic<T, Self::Reclaimer, N>,
        expected: MarkedPtr<T, N>,
        order: Ordering,
    ) -> AcquireResult<'g, T, Self::Reclaimer, N> {
        self.protect_if_equal(atomic, expected, order)
    }
}

/********** impl blanket (ProtectRegion) **********************************************************/

impl<'g, G> GuardRef<'g> for &'g G
where
    G: ProtectRegion,
{
    type Reclaimer = G::Reclaimer;

    #[inline]
    fn load_protected<T, N: Unsigned>(
        self,
        atomic: &Atomic<T, Self::Reclaimer, N>,
        order: Ordering,
    ) -> MarkedOption<Shared<'g, T, Self::Reclaimer, N>> {
        atomic
            .load_unprotected_marked_option(order)
            .map(|unprotected| unsafe { unprotected.into_shared() })
    }

    #[inline]
    fn load_protected_if_equal<T, N: Unsigned>(
        self,
        atomic: &Atomic<T, Self::Reclaimer, N>,
        expected: MarkedPtr<T, N>,
        order: Ordering,
    ) -> AcquireResult<'g, T, Self::Reclaimer, N> {
        atomic.load_raw_if_equal(expected, order).map(|ptr| {
            MarkedOption::from(ptr).map(|ptr| Shared { inner: ptr, _marker: PhantomData })
        })
    }
}

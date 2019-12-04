use core::marker::PhantomData;
use core::sync::atomic::Ordering;

use conquer_pointer::{MarkedPtr, MaybeNull};
use typenum::Unsigned;

use crate::atomic::Atomic;
use crate::traits::{Protect, ProtectRegion, Reclaimer};
use crate::{NotEqualError, Shared};

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
    ) -> MaybeNull<Shared<'g, T, Self::Reclaimer, N>>;

    /// TODO: Docs...
    fn load_protected_if_equal<T, N: Unsigned>(
        self,
        atomic: &Atomic<T, Self::Reclaimer, N>,
        expected: MarkedPtr<T, N>,
        order: Ordering,
    ) -> Result<MaybeNull<Shared<'g, T, Self::Reclaimer, N>>, NotEqualError>;
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
    ) -> MaybeNull<Shared<'g, T, Self::Reclaimer, N>> {
        self.protect(atomic, order)
    }

    #[inline]
    fn load_protected_if_equal<T, N: Unsigned>(
        self,
        atomic: &Atomic<T, Self::Reclaimer, N>,
        expected: MarkedPtr<T, N>,
        order: Ordering,
    ) -> Result<MaybeNull<Shared<'g, T, Self::Reclaimer, N>>, NotEqualError> {
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
    ) -> MaybeNull<Shared<'g, T, Self::Reclaimer, N>> {
        MaybeNull::from(atomic.load_raw(order)).map(|inner| Shared { inner, _marker: PhantomData })
    }

    #[inline]
    fn load_protected_if_equal<T, N: Unsigned>(
        self,
        atomic: &Atomic<T, Self::Reclaimer, N>,
        expected: MarkedPtr<T, N>,
        order: Ordering,
    ) -> Result<Shared<'g, T, Self::Reclaimer, N>, NotEqualError> {
        atomic
            .load_raw_if_equal(expected, order)
            .map(|ptr| Shared { inner: ptr, _marker: PhantomData })
    }
}

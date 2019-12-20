//! TODO: mod-level docs

use core::marker::PhantomData;
use core::sync::atomic::Ordering;

use conquer_pointer::{
    MarkedPtr,
    MaybeNull::{self, NotNull, Null},
};

use crate::retired::Retired;
use crate::traits::{GlobalReclaim, LocalRef, Protect, ProtectRegion, Reclaim};
use crate::typenum::Unsigned;
use crate::NotEqualError;

/// A specialization of the [`Atomic`](crate::atomic::Atomic) type using
/// [`Leaking`] as reclaimer.
pub type Atomic<T, N> = crate::atomic::Atomic<T, Leaking, N>;
/// A specialization of the [`Owned`](crate::Owned) type using [`Leaking`] as
/// reclaimer.
pub type Owned<T, N> = crate::Owned<T, Leaking, N>;
/// A specialization of the [`Shared`](crate::Shared) type using [`Leaking`] as
/// reclaimer.
pub type Shared<'g, T, N> = crate::Shared<'g, T, Leaking, N>;
/// A specialization of the [`Unlinked`](crate::Unlinked) type using [`Leaking`]
/// as reclaimer.
pub type Unlinked<T, N> = crate::Unlinked<T, Leaking, N>;
/// A specialization of the [`Unprotected`](crate::Unprotected) type using
/// [`Leaking`] as reclaimer.
pub type Unprotected<T, N> = crate::Unprotected<T, Leaking, N>;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Leaking
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A no-op [`GlobalReclaimer`] that deliberately leaks memory.
#[derive(Copy, Clone, Debug, Default, Hash, Eq, Ord, PartialEq, PartialOrd)]
pub struct Leaking;

/********** impl GlobalReclaim ********************************************************************/

impl GlobalReclaim for Leaking {
    #[inline]
    fn build_local_ref() -> Self::Ref {
        Handle
    }
}

/********** impl Reclaimer ************************************************************************/

unsafe impl Reclaim for Leaking {
    type Header = ();
    type Ref = Handle;

    #[inline]
    fn new() -> Self {
        Leaking
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Handle
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: docs...
#[derive(Copy, Clone, Default, Debug, Hash, Eq, Ord, PartialEq, PartialOrd)]
pub struct Handle;

/********** impl ReclaimHandle ********************************************************************/

unsafe impl LocalRef for Handle {
    type Guard = Guard;
    type Reclaimer = Leaking;

    fn from_ref(global: &Self::Reclaimer) -> Self {
        Handle
    }

    unsafe fn from_raw(global: *const Self::Reclaimer) -> Self {
        Handle
    }

    fn into_guard(self) -> Self::Guard {
        Guard
    }

    #[inline]
    unsafe fn retire(self, _: Retired<Self::Reclaimer>) {}
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Guard
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: docs...
#[derive(Debug, Default, Copy, Clone, Eq, Ord, PartialEq, PartialOrd)]
pub struct Guard;

/********** impl Protect **************************************************************************/

unsafe impl Protect for Guard {
    type Reclaimer = Leaking;

    #[inline]
    fn release(&mut self) {}

    #[inline]
    fn protect<T, N: Unsigned + 'static>(
        &mut self,
        atomic: &Atomic<T, N>,
        order: Ordering,
    ) -> MaybeNull<Shared<T, N>> {
        MaybeNull::from(atomic.load_raw(order))
            .map(|ptr| Shared { inner: ptr, _marker: PhantomData })
    }

    #[inline]
    fn protect_if_equal<T, N: Unsigned + 'static>(
        &mut self,
        atomic: &Atomic<T, N>,
        expected: MarkedPtr<T, N>,
        order: Ordering,
    ) -> Result<MaybeNull<Shared<T, N>>, NotEqualError> {
        atomic.load_raw_if_equal(expected, order).map(|ptr| match ptr {
            NotNull(inner) => NotNull(Shared { inner, _marker: PhantomData }),
            Null(tag) => Null(tag),
        })
    }
}

/********** impl ProtectRegion ********************************************************************/

unsafe impl ProtectRegion for Guard {}

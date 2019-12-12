//! TODO: mod-level docs

use core::marker::PhantomData;
use core::sync::atomic::Ordering;

use conquer_pointer::{
    MarkedPtr,
    MaybeNull::{self, NotNull, Null},
};

use crate::retired::Retired;
use crate::traits::{
    GlobalReclaimer, OwningReclaimer, Protect, ProtectRegion, Reclaimer, ReclaimerHandle,
};
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

unsafe impl GlobalReclaimer for Leaking {
    #[inline]
    fn handle() -> Self::Handle {
        Handle
    }

    #[inline]
    fn guard() -> <Self::Handle as ReclaimerHandle>::Guard {
        Guard
    }

    #[inline]
    unsafe fn retire(_: Retired<Self>) {}
}

/********** impl GenericReclaimer *****************************************************************/

unsafe impl OwningReclaimer for Leaking {
    type Handle = Handle;

    #[inline]
    fn owning_local_handle(&self) -> Self::Handle {
        Handle
    }
}

/********** impl Reclaimer ************************************************************************/

unsafe impl Reclaimer for Leaking {
    type Global = ();
    type Header = ();

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

unsafe impl ReclaimerHandle for Handle {
    type Reclaimer = Leaking;
    type Guard = Guard;

    #[inline]
    fn guard(self) -> Self::Guard {
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
    fn protect<T, N: Unsigned>(
        &mut self,
        atomic: &Atomic<T, N>,
        order: Ordering,
    ) -> MaybeNull<Shared<T, N>> {
        MaybeNull::from(atomic.load_raw(order))
            .map(|ptr| Shared { inner: ptr, _marker: PhantomData })
    }

    #[inline]
    fn protect_if_equal<T, N: Unsigned>(
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

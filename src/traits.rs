use core::sync::atomic::Ordering;

use conquer_pointer::{MarkedPtr, MaybeNull};

use crate::atomic::Atomic;
use crate::retired::Retired;
use crate::typenum::Unsigned;
use crate::{NotEqualError, Shared};

////////////////////////////////////////////////////////////////////////////////////////////////////
// GlobalReclaimer (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub unsafe trait GlobalReclaimer
where
    Self: GenericReclaimer,
    <Self as GenericReclaimer>::Handle: Default,
{
    fn guard() -> <Self::Handle as ReclaimerHandle>::Guard;
    unsafe fn retire(record: Retired<Self>);
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// GenericReclaimer (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub unsafe trait GenericReclaimer: Reclaimer {
    type Handle: ReclaimerHandle<Reclaimer = Self>;

    fn create_local_handle(&self) -> Self::Handle;
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Reclaimer (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub unsafe trait Reclaimer: Default + Sync + Sized + 'static {
    type Global: Default + Sync + Sized;

    type Header: Default + Sync + Sized + 'static;
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ReclaimerHandle (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub unsafe trait ReclaimerHandle: Clone + Sized {
    type Reclaimer: Reclaimer;
    type Guard: Protect<Reclaimer = Self::Reclaimer>;

    fn guard(self) -> Self::Guard;
    unsafe fn retire(self, record: Retired<Self::Reclaimer>);
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Protect (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub unsafe trait Protect: Clone + Sized {
    /// The associated memory reclaimer.
    type Reclaimer: Reclaimer;

    /// Releases any protection that may be provided by the guard.
    ///
    /// This method has no effect for guards that also implement
    /// [`ProtectRegion`].
    fn release(&mut self);

    fn protect<T, N: Unsigned>(
        &mut self,
        src: &Atomic<T, Self::Reclaimer, N>,
        order: Ordering,
    ) -> MaybeNull<Shared<T, Self::Reclaimer, N>>;

    fn protect_if_equal<T, N: Unsigned>(
        &mut self,
        src: &Atomic<T, Self::Reclaimer, N>,
        expected: MarkedPtr<T, N>,
        order: Ordering,
    ) -> Result<MaybeNull<Shared<T, Self::Reclaimer, N>>, NotEqualError>;
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ProtectRegion (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub unsafe trait ProtectRegion: Protect {}

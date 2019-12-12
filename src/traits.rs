use core::sync::atomic::Ordering;

use conquer_pointer::{MarkedPtr, MaybeNull};

use crate::atomic::Atomic;
use crate::retired::Retired;
use crate::typenum::Unsigned;
use crate::{NotEqualError, Shared};

////////////////////////////////////////////////////////////////////////////////////////////////////
// GlobalReclaimer (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub unsafe trait GlobalReclaimer: GenericReclaimer {
    fn handle() -> Self::Handle;
    fn guard() -> <Self::Handle as ReclaimerHandle>::Guard;
    unsafe fn retire(record: Retired<Self>);
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// GenericReclaimer (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub unsafe trait GenericReclaimer: Reclaimer {
    type Handle: ReclaimerHandle<Reclaimer = Self>;

    fn local_handle(&self) -> Self::Handle;
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Reclaimer (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub unsafe trait Reclaimer: Default + Sync + Sized + 'static {
    type Global: Default + Sync + Sized;
    // TODO: type Handle<'local, 'global>: ReclaimerHandle<Reclaimer = Self> + 'local + 'global;
    type Header: Default + Sync + Sized + 'static;

    fn new() -> Self;
    // fn owning_local_handle<'global>(&'global self) -> Self::Handle<'_, 'global>;
    // unsafe fn raw_local_handle(&self) -> Self::Handle<'_, '_>;
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

/// A trait for implementing guard types associated with a specific
/// [`Reclaimer`].
pub unsafe trait Protect: Clone + Sized {
    /// The associated memory reclaimer.
    type Reclaimer: Reclaimer;

    /// Releases any protection that may be provided by the guard.
    ///
    /// This method has no effect for guards that also implement
    /// [`ProtectRegion`].
    fn release(&mut self);

    /// Loads and protects the value currently stored in `atomic` and returns
    /// an optional protected [`Shared`] reference.
    ///
    /// `protect` takes an [`Ordering`] argument ...
    fn protect<T, N: Unsigned>(
        &mut self,
        src: &Atomic<T, Self::Reclaimer, N>,
        order: Ordering,
    ) -> MaybeNull<Shared<T, Self::Reclaimer, N>>;

    /// Loads and protects the value currently stored in `atomic` if it equals
    /// the `expected` pointer and returns an optional protected [`Shared`]
    /// reference.
    ///
    /// `protect_if_equal` takes and [`Ordering`] argument ...
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

/// An extension trait for guard types of [`Reclaimer`]s that protect entire
/// regions of code rather than individual [`Atomic`] pointers.
pub unsafe trait ProtectRegion: Protect {}

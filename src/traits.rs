use core::sync::atomic::Ordering;

use conquer_pointer::{MarkedPtr, MaybeNull};

use crate::atomic::Atomic;
use crate::retired::Retired;
use crate::typenum::Unsigned;
use crate::{NotEqualError, Shared};

////////////////////////////////////////////////////////////////////////////////////////////////////
// GlobalReclaim (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait GlobalReclaim: Reclaim {
    fn build_local_ref() -> Self::Ref;

    fn build_guard() -> <Self::Ref as ReclaimerLocalRef>::Guard {
        Self::build_local_ref().into_guard()
    }

    unsafe fn retire(retired: Retired<Self>) {
        Self::build_local_ref().retire(retired);
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Reclaim (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub unsafe trait Reclaim: Default + Sync + Sized + 'static {
    type Header: Default + Sync + Sized + 'static;
    type Ref: ReclaimerLocalRef<Reclaimer = Self>;
    // type Ref<'global>: LocalRef<Reclaimer = Self> + 'global;

    fn new() -> Self;
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// LocalRef (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub unsafe trait ReclaimerLocalRef: Clone + Sized {
    type Guard: Protect<Reclaimer = Self::Reclaimer>;
    type Reclaimer: Reclaim;

    fn from_ref(global: &Self::Reclaimer) -> Self;
    unsafe fn from_raw(global: &Self::Reclaimer) -> Self;

    fn into_guard(self) -> Self::Guard;
    unsafe fn retire(self, retired: Retired<Self::Reclaimer>);
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Protect (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A trait for implementing guard types associated with a specific
/// [`Reclaimer`].
pub unsafe trait Protect: Clone + Sized {
    /// The associated memory reclaimer.
    type Reclaimer: Reclaim;

    /// Releases any protection that may be provided by the guard.
    ///
    /// This method has no effect for guards that also implement
    /// [`ProtectRegion`].
    fn release(&mut self);

    /// Loads and protects the value currently stored in `atomic` and returns
    /// an optional protected [`Shared`] reference.
    ///
    /// `protect` takes an [`Ordering`] argument ...
    fn protect<T, N: Unsigned + 'static>(
        &mut self,
        src: &Atomic<T, Self::Reclaimer, N>,
        order: Ordering,
    ) -> MaybeNull<Shared<T, Self::Reclaimer, N>>;

    /// Loads and protects the value currently stored in `atomic` if it equals
    /// the `expected` pointer and returns an optional protected [`Shared`]
    /// reference.
    ///
    /// `protect_if_equal` takes and [`Ordering`] argument ...
    fn protect_if_equal<T, N: Unsigned + 'static>(
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

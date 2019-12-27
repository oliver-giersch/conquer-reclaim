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
    fn build_global_ref() -> Self::Ref;

    fn build_guard() -> <Self::Ref as ReclaimRef>::Guard {
        Self::build_global_ref().into_guard()
    }

    unsafe fn retire(retired: Retired<Self>) {
        Self::build_global_ref().retire(retired);
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Reclaim (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub unsafe trait Reclaim: Default + Sync + Sized + 'static {
    type Header: Default + Sync + Sized + 'static;
    type Ref: ReclaimRef<Reclaimer = Self> + BuildReclaimRef<'static>;
    // type Ref<'global>: ReclaimRef<Reclaimer = Self> + BuildReclaimRef<'a>;

    fn new() -> Self;
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ReclaimRef (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub unsafe trait ReclaimRef: Clone + Sized {
    type Guard: Protect<Reclaimer = Self::Reclaimer>;
    type Reclaimer: Reclaim;

    fn from_ref<'global>(global: &'global Self::Reclaimer) -> Self
    where
        Self: BuildReclaimRef<'global>,
    {
        <Self as BuildReclaimRef<'_>>::from_ref(global)
    }

    /// Creates a new [`ReclaimRef`] from a raw pointer to the ...
    unsafe fn from_raw(global: &Self::Reclaimer) -> Self
    where
        Self: 'static;

    fn into_guard(self) -> Self::Guard;
    unsafe fn retire(self, retired: Retired<Self::Reclaimer>);
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// BuildReclaimRef (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait BuildReclaimRef<'global>: ReclaimRef + 'global {
    fn from_ref(global: &'global Self::Reclaimer) -> Self;
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

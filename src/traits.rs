use core::sync::atomic::Ordering;

use conquer_pointer::typenum::Unsigned;
use conquer_pointer::MarkedPtr;

use crate::atomic::Atomic;
use crate::retired::Retired;
use crate::{NotEqual, Protected};

////////////////////////////////////////////////////////////////////////////////////////////////////
// ReclaimRef (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ReclaimRef<T>: Sized {
    type LocalState: ReclaimLocalState<T, Reclaim = Self::Reclaim>;
    type Reclaim: Reclaim + Retire<T>;

    unsafe fn build_local_state(&self) -> Self::LocalState;
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ReclaimLocalState (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ReclaimLocalState<T> {
    type Guard: Protect<T, Reclaim = Self::Reclaim>;
    type Reclaim: Reclaim + Retire<T>;

    fn build_guard(&self) -> Self::Guard;
    unsafe fn retire_record(&self, retired: Retired<Self::Reclaim>);
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Protect (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A trait for implementing guard types associated with a specific [`Reclaim`]
/// mechanism.
pub unsafe trait Protect<T>: Clone {
    /// The associated [`Reclaim`] mechanism.
    type Reclaim: Reclaim + Retire<T>;

    /// Loads and protects the value currently stored in `src` and returns a
    /// protected [`Shared`] pointer to it.
    ///
    /// `protect` takes an [`Ordering`] argument...
    fn protect<N: Unsigned>(
        &mut self,
        atomic: &Atomic<T, Self::Reclaim, N>,
        order: Ordering,
    ) -> Protected<T, Self::Reclaim, N>;

    /// Loads and protects the value currently stored in `src` if it equals the
    /// `expected` value and returns a protected [`Shared`] pointer to it.
    ///
    /// `protect_if_equal` takes an [`Ordering`] argument...
    fn protect_if_equal<N: Unsigned>(
        &mut self,
        atomic: &Atomic<T, Self::Reclaim, N>,
        expected: MarkedPtr<T, N>,
        order: Ordering,
    ) -> Result<Protected<T, Self::Reclaim, N>, NotEqual>;
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Retire (public sealed trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait Retire<T>: Reclaim {
    unsafe fn retire(ptr: *mut T) -> *mut Self::Reclaimable;
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Reclaim (public sealed trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait Reclaim: Sealed + Sized {
    type DropCtx: Default + Sized;
    type Header: Default + Sized;
    type Reclaimable: Sized;

    const VIRTUAL_DROP: bool;

    unsafe fn reclaim(retired: *mut Self::Reclaimable);
    unsafe fn convert_to_data(retired: *mut Self::Reclaimable) -> *mut ();
    unsafe fn convert_to_header(retired: *mut Self::Reclaimable) -> *mut Self::Header;
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Sealed (private sealed trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait Sealed {}

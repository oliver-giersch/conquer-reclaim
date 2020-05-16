use core::sync::atomic::Ordering;

use conquer_pointer::typenum::Unsigned;
use conquer_pointer::MarkedPtr;

use crate::atomic::Atomic;
use crate::retired::Retired;
use crate::{NotEqual, Protected};

////////////////////////////////////////////////////////////////////////////////////////////////////
// ReclaimRef (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ReclaimRef: Sized {
    type LocalState: ReclaimLocalState<Reclaim = Self::Reclaim>;
    type Reclaim: Reclaim;

    unsafe fn build_local_state(&self) -> Self::LocalState;
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ReclaimLocalState (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ReclaimLocalState {
    type Guard: Protect<Reclaim = Self::Reclaim>;
    type Reclaim: Reclaim;

    fn build_guard(&self) -> Self::Guard;
    unsafe fn retire_record(&self, retired: Retired<Self::Reclaim>);
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Protect (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A trait for implementing guard types associated with a specific [`Reclaim`]
/// mechanism.
pub unsafe trait Protect: Clone {
    /// The associated [`Reclaim`] mechanism.
    type Reclaim: Reclaim;

    /// Loads and protects the value currently stored in `src` and returns a
    /// protected [`Shared`] pointer to it.
    ///
    /// `protect` takes an [`Ordering`] argument...
    fn protect<T, N: Unsigned>(
        &mut self,
        atomic: &Atomic<T, Self::Reclaim, N>,
        order: Ordering,
    ) -> Protected<T, Self::Reclaim, N>
    where
        Self::Reclaim: Retire<T>;

    /// Loads and protects the value currently stored in `src` if it equals the
    /// `expected` value and returns a protected [`Shared`] pointer to it.
    ///
    /// `protect_if_equal` takes an [`Ordering`] argument...
    fn protect_if_equal<T, N: Unsigned>(
        &mut self,
        atomic: &Atomic<T, Self::Reclaim, N>,
        expected: MarkedPtr<T, N>,
        order: Ordering,
    ) -> Result<Protected<T, Self::Reclaim, N>, NotEqual>
    where
        Self::Reclaim: Retire<T>;
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Retire (public sealed trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub unsafe trait Retire<T>: Reclaim {
    unsafe fn retire(ptr: *mut T) -> *mut Self::Reclaimable;
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Reclaim
////////////////////////////////////////////////////////////////////////////////////////////////////

pub unsafe trait Reclaim: Sized {
    type DropCtx: Default + Sized;
    type Header: Default + Sized;
    type Reclaimable: Sized;

    unsafe fn reclaim(retired: *mut Self::Reclaimable);
    unsafe fn convert_to_data(retired: *mut Self::Reclaimable) -> *mut ();
    unsafe fn convert_to_header(retired: *mut Self::Reclaimable) -> *mut Self::Header;
}

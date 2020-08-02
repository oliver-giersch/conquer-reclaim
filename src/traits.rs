use core::sync::atomic::Ordering;

#[cfg(not(feature = "std"))]
use alloc::boxed::Box;

use conquer_pointer::typenum::Unsigned;
use conquer_pointer::MarkedPtr;

//use crate::alias::AssocReclaimBase;
use crate::atomic::Atomic;
use crate::{NotEqual, Owned, Protected, Retired};

////////////////////////////////////////////////////////////////////////////////////////////////////
// ReclaimBase (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub unsafe trait ReclaimBase {
    type Header: Sized;
    type Retired: ?Sized;
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Reclaim (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub unsafe trait Reclaim<T>: ReclaimBase {
    unsafe fn retire(ptr: *mut T) -> *mut Self::Retired;
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ReclaimRef (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub unsafe trait ReclaimRef<T> {
    type Reclaim: Reclaim<T>;
    type ThreadState: ReclaimThreadState<T, Reclaim = Self::Reclaim>;

    fn alloc_owned<N: Unsigned>(&self, value: T) -> Owned<T, Self::Reclaim, N>;
    unsafe fn build_thread_state_unchecked(&self) -> Self::ThreadState;
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ReclaimThreadState (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub unsafe trait ReclaimThreadState<T> {
    type Reclaim: Reclaim<T>;
    type Guard: Protect<T, Reclaim = Self::Reclaim>;

    fn build_guard(&self) -> Self::Guard;
    fn alloc_owned<N: Unsigned>(&self, value: T) -> Owned<T, Self::Reclaim, N>;
    unsafe fn retire_record(&self, retired: Retired<Self::Reclaim>);
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Protect (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A trait for implementing guard types associated with a specific [`Reclaim`]
/// mechanism.
pub unsafe trait Protect<T>: Clone {
    /// The associated [`Reclaim`] mechanism.
    type Reclaim: Reclaim<T>;

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

use core::sync::atomic::Ordering;

use conquer_pointer::typenum::Unsigned;
use conquer_pointer::MarkedPtr;

use crate::atomic::Atomic;
use crate::{NotEqual, Owned, Protected, Unlinked};

////////////////////////////////////////////////////////////////////////////////////////////////////
// AssocReclaim (type alias)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub type AssocReclaim<R> = <R as ReclaimRef>::Reclaim;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Reclaim (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub unsafe trait Reclaim: Sized {
    type Header: Sized;
    type Retired: Sized;

    unsafe fn reclaim(retired: *mut Self::Retired);
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Retire (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub unsafe trait Retire<T>: Reclaim {
    unsafe fn retire(ptr: *mut T) -> *mut Self::Retired;
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ReclaimRef (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub unsafe trait ReclaimRef: Sized {
    type Item: Sized;
    type Reclaim: Reclaim + Retire<Self::Item>;

    type LocalState: ReclaimLocalState<Item = Self::Item, Reclaim = Self::Reclaim>;

    fn alloc_owned<N: Unsigned>(&self, value: Self::Item) -> Owned<Self::Item, Self::Reclaim, N>;
    unsafe fn build_local_state(&self) -> Self::LocalState;
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ReclaimLocalState (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ReclaimLocalState {
    type Item: Sized;
    type Reclaim: Reclaim;

    type Guard: Protect<Item = Self::Item, Reclaim = Self::Reclaim>;

    fn alloc_owned<N: Unsigned>(&self, value: Self::Item) -> Owned<Self::Item, Self::Reclaim, N>;
    fn build_guard(&self) -> Self::Guard;
    unsafe fn retire_record<N: Unsigned>(&self, unlinked: Unlinked<Self::Item, Self::Reclaim, N>);
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Protect (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A trait for implementing guard types associated with a specific [`Reclaim`]
/// mechanism.
pub unsafe trait Protect: Clone {
    type Item;
    /// The associated [`Reclaim`] mechanism.
    type Reclaim: Reclaim;

    /// Loads and protects the value currently stored in `src` and returns a
    /// protected [`Shared`] pointer to it.
    ///
    /// `protect` takes an [`Ordering`] argument...
    fn protect<N: Unsigned>(
        &mut self,
        atomic: &Atomic<Self::Item, Self::Reclaim, N>,
        order: Ordering,
    ) -> Protected<Self::Item, Self::Reclaim, N>;

    /// Loads and protects the value currently stored in `src` if it equals the
    /// `expected` value and returns a protected [`Shared`] pointer to it.
    ///
    /// `protect_if_equal` takes an [`Ordering`] argument...
    fn protect_if_equal<N: Unsigned>(
        &mut self,
        atomic: &Atomic<Self::Item, Self::Reclaim, N>,
        expected: MarkedPtr<Self::Item, N>,
        order: Ordering,
    ) -> Result<Protected<Self::Item, Self::Reclaim, N>, NotEqual>;
}

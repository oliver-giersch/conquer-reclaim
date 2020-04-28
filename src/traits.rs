use core::sync::atomic::Ordering;

use conquer_pointer::typenum::Unsigned;
use conquer_pointer::MarkedPtr;

use crate::atomic::Atomic;
use crate::{NotEqual, Protected, Retired};

////////////////////////////////////////////////////////////////////////////////////////////////////
// AssocItem
////////////////////////////////////////////////////////////////////////////////////////////////////

pub type AssocItem<R> = <<R as Reclaim>::Strategy as ReclaimStrategy>::Item;

////////////////////////////////////////////////////////////////////////////////////////////////////
// GlobalReclaim (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub unsafe trait GlobalReclaim: Reclaim {
    fn build_guard() -> <Self::LocalState as LocalState>::Guard {
        <Self as GlobalReclaim>::build_local_state().build_guard()
    }

    unsafe fn retire_record(retired: Retired<Self>) {
        <Self as GlobalReclaim>::build_local_state().retire_record(retired)
    }

    fn build_local_state() -> Self::LocalState;
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Reclaim (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait Reclaim: Default + Send + Sync + Sized + 'static {
    type Header: Default + Sized;
    type LocalState: LocalState<Reclaimer = Self> + 'static;
    type Strategy: ReclaimStrategy;

    unsafe fn build_local_state(&self) -> Self::LocalState;
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Retire (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A trait that signifies that an implementing [`Reclaim`] mechanism is capable
/// of retiring instances of type `T`.
pub unsafe trait Retire<T>: Reclaim {
    unsafe fn convert(retired: *mut T) -> *mut AssocItem<Self>;
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// LocalState (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A trait for references to thread-local state instances of a specific
/// [`Reclaim`] mechanism.
///
/// Thread-local state references should in general be cheap to construct and
/// have to be cloneable.
/// Their two primary purposes are:
///
/// - creating new guard instances (which implement the [`Protect`] trait)
/// - retiring records
pub unsafe trait LocalState: Sized {
    /// The associated [`Protect`] type.
    type Guard: Protect<Reclaimer = Self::Reclaimer>;
    /// The associated [`Reclaim`] mechanism.
    type Reclaimer: Reclaim;

    /// Creates a new guard instance.
    fn build_guard(&self) -> Self::Guard;
    /// Retires the given record.
    ///
    /// # Safety
    ///
    /// TODO
    unsafe fn retire_record(&self, retired: Retired<Self::Reclaimer>);
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Protect (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A trait for implementing guard types associated with a specific [`Reclaim`]
/// mechanism.
pub unsafe trait Protect: Clone {
    /// The associated [`Reclaim`] mechanism.
    type Reclaimer: Reclaim;

    /// Loads and protects the value currently stored in `src` and returns a
    /// protected [`Shared`] pointer to it.
    ///
    /// `protect` takes an [`Ordering`] argument...
    fn protect<T, N: Unsigned>(
        &mut self,
        atomic: &Atomic<T, Self::Reclaimer, N>,
        order: Ordering,
    ) -> Protected<T, Self::Reclaimer, N>
    where
        Self::Reclaimer: Retire<T>;

    /// Loads and protects the value currently stored in `src` if it equals the
    /// `expected` value and returns a protected [`Shared`] pointer to it.
    ///
    /// `protect_if_equal` takes an [`Ordering`] argument...
    fn protect_if_equal<T, N: Unsigned>(
        &mut self,
        atomic: &Atomic<T, Self::Reclaimer, N>,
        expected: MarkedPtr<T, N>,
        order: Ordering,
    ) -> Result<Protected<T, Self::Reclaimer, N>, NotEqual>
    where
        Self::Reclaimer: Retire<T>;
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ReclaimStrategy (sealed trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ReclaimStrategy {
    type DropCtx: Default + Sized;
    type Item: Sized;

    unsafe fn reclaim(retired: *mut Self::Item);
}

use core::sync::atomic::Ordering;

use conquer_pointer::typenum::Unsigned;
use conquer_pointer::MarkedPtr;

use crate::atomic::Atomic;
use crate::retired::AssocRetired;
use crate::{NotEqual, Protected};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Reclaim (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait Reclaim: Default + Send + Sync + Sized + 'static {
    type Item: Sized;
    type Header: Default + Sized;
    type LocalState: LocalState<Reclaimer = Self> + 'static;

    unsafe fn build_local_state(&self) -> Self::LocalState;
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
    unsafe fn retire_record(&self, retired: AssocRetired<Self::Reclaimer>);
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
    ) -> Protected<T, Self::Reclaimer, N>;

    /// Loads and protects the value currently stored in `src` if it equals the
    /// `expected` value and returns a protected [`Shared`] pointer to it.
    ///
    /// `protect_if_equal` takes an [`Ordering`] argument...
    fn protect_if_equal<T, N: Unsigned>(
        &mut self,
        atomic: &Atomic<T, Self::Reclaimer, N>,
        expected: MarkedPtr<T, N>,
        order: Ordering,
    ) -> Result<Protected<T, Self::Reclaimer, N>, NotEqual>;
}

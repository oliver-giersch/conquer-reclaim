//! TODO: crate lvl docs...

#![cfg_attr(not(any(test, feature = "std")), no_std)]
//#![warn(missing_docs)]

#[cfg(not(feature = "std"))]
extern crate alloc;

pub mod prelude {
    //! TODO: docs...

    pub use crate::traits::{
        GlobalReclaimer, Protect, ProtectRegion, Reclaimer, ReclaimerHandle, SharedPointer,
    };
}

#[macro_use]
mod internal;

pub mod leak;

mod atomic;
mod guarded;
mod imp;
mod record;
mod retired;
mod traits;

use core::marker::PhantomData;

pub use conquer_pointer;
pub use conquer_pointer::typenum;

use conquer_pointer::{MarkedNonNull, MarkedOption};
use typenum::Unsigned;

pub use crate::atomic::{Atomic, CompareExchangeError};
pub use crate::guarded::Guarded;
pub use crate::record::Record;
pub use crate::retired::Retired;
pub use crate::traits::{
    GlobalReclaimer, Protect, ProtectRegion, Reclaimer, ReclaimerHandle, SharedPointer,
};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Owned (impl in imp/owned.rs)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A pointer type for heap allocated values similar to [`Box`].
///
/// `Owned` values function like marked pointers and are also guaranteed to
/// allocate the appropriate [`RecordHeader`][Reclaim::RecordHeader] type
/// for its generic [`Reclaim`] parameter alongside their actual content.
#[derive(Eq, Ord, PartialEq, PartialOrd)]
pub struct Owned<T, R: Reclaimer, N: Unsigned> {
    inner: MarkedNonNull<T, N>,
    _marker: PhantomData<(T, R)>,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Shared (impl in imp/shared.rs)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A shared reference to a value that is actively protected from reclamation by
/// other threads.
///
/// `Shared` values have similar semantics to shared references (`&'g T`), i.e.
/// they can be trivially copied, cloned and (safely) de-referenced.
/// However, they do retain potential mark bits of the atomic value from which
/// they were originally read.
/// They are also usually borrowed from guard values implementing the
/// [`Protect`] trait.
pub struct Shared<'g, T, R, N> {
    inner: MarkedNonNull<T, N>,
    _marker: PhantomData<(&'g T, R)>,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Unlinked (impl in imp/unlinked.rs)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A reference to a value that has been removed from its previous location in
/// memory and is hence no longer reachable by other threads.
///
/// `Unlinked` values are the result of (successful) atomic *swap* or
/// *compare-and-swap* operations on [`Atomic`] values.
/// They are move-only types, but they don't have full ownership semantics,
/// either.
/// Dropping an `Unlinked` value without explicitly retiring it almost certainly
/// results in a memory leak.
///
/// The safety invariants around retiring `Unlinked` references are explained
/// in detail in the documentation for [`retire_local`][Reclaim::retire_local].
#[derive(Eq, Ord, PartialEq, PartialOrd)]
#[must_use = "unlinked values are meant to be retired, otherwise a memory leak is highly likely"]
pub struct Unlinked<T, R, N> {
    inner: MarkedNonNull<T, N>,
    _marker: PhantomData<(T, R)>,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Unprotected (impl in imp/unprotected.rs)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A reference to a value loaded from an [`Atomic`] that is not actively
/// protected from reclamation.
///
/// `Unprotected` values can not be safely de-referenced under usual
/// circumstances (i.e. other threads can retire and reclaim unlinked records).
/// They do, however, have stronger guarantees than raw (marked) pointers:
/// Since are loaded from [`Atomic`] values they must (at least at one point)
/// have been *valid* references.
#[derive(Eq, Ord, PartialEq, PartialOrd)]
pub struct Unprotected<T, R, N> {
    inner: MarkedNonNull<T, N>,
    _marker: PhantomData<R>,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// NotEqualError
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: Docs...
#[derive(Debug, Copy, Clone, Eq, Ord, PartialEq, PartialOrd)]
pub struct NotEqualError(());

////////////////////////////////////////////////////////////////////////////////////////////////////
// AcquireResult
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Result type the [`acquire_if_equal`][crate::traits::Protect::acquire_if_equal]
/// trait method.
pub type AcquireResult<'g, T, R, N> = Result<MarkedOption<Shared<'g, T, R, N>>, NotEqualError>;

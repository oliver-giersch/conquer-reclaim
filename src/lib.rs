//! TODO: crate lvl docs...

#![cfg_attr(not(any(test, feature = "std")), no_std)]
#![warn(missing_docs)]

#[cfg(not(feature = "std"))]
extern crate alloc;

pub mod prelude {
    //! TODO: docs...

    pub use conquer_pointer::MaybeNull::{self, NotNull, Null};

    pub use crate::traits::{
        BuildReclaimRef, GlobalReclaim, Protect, ProtectRegion, Reclaim, ReclaimRef,
    };
}

#[macro_use]
mod macros;

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

use conquer_pointer::{MarkedNonNull, MarkedPtr};

pub use crate::atomic::{Atomic, CompareExchangeError};
pub use crate::guarded::Guarded;
pub use crate::record::Record;
pub use crate::retired::{RawRetired, Retired};
pub use crate::traits::{
    BuildReclaimRef, GlobalReclaim, Protect, ProtectRegion, Reclaim, ReclaimRef,
};

use crate::typenum::Unsigned;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Owned (impl in imp/owned.rs)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A safe smart pointer type for heap allocated values similar to [`Box`].
///
/// Unlike [`Box`], `Owned` instances support pointer tagging and are tightly
/// bound to their associated [`Reclaim`] type.
/// Specifically, on creation they invariably allocate the associated
/// [`Header`][Reclaim::Header] alongside the actual value (see also
/// [`Record`]).
#[derive(Eq, Ord, PartialEq, PartialOrd)]
pub struct Owned<T, R: Reclaim, N: Unsigned + 'static> {
    inner: MarkedNonNull<T, N>,
    _marker: PhantomData<(T, R)>,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Shared (impl in imp/shared.rs)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A local shared reference to a protected value that supports pointer tagging.
///
/// Instances of `Shared` are derived from guard types (see [`Protect`] and
/// [`ProtectRegion`]) from which they inherit their lifetime dependence.
/// Like regular shared references (`&'g T`) they can be trivially copied,
/// cloned, de-referenced and can not be `null`.
///
/// See the documentation for [`deref`][Shared::deref] for an explanation of the
/// safety concerns involved in de-referencing a `Shared`.
pub struct Shared<'g, T, R, N> {
    inner: MarkedNonNull<T, N>,
    _marker: PhantomData<(&'g T, R)>,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Unlinked (impl in imp/unlinked.rs)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A reference type for a value that has been removed from its previous
/// location as the result of a [`swap`][swap] or [`compare_exchange`][cex]
/// operation on an [`Atomic`].
///
/// Under the assumption that no other thread is able to load a *new* reference
/// to the same value as the `Unlinked` after it being unlinked, it is sound to
/// [`retire`][Unlinked::retire] the value, which hands it over to the
/// [`Reclaim`] mechanism for eventual de-allocation.
/// This is for instance always the case if the [`Atomic`] was a unique pointer
/// to the unlinked value.
///
/// [swap]: Atomic::swap
/// [cex]: Atomic::compare_exchange
#[derive(Eq, Ord, PartialEq, PartialOrd)]
#[must_use = "unlinked values are meant to be retired, otherwise a memory leak is highly likely"]
pub struct Unlinked<T, R, N> {
    inner: MarkedNonNull<T, N>,
    _marker: PhantomData<(T, R)>,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Unprotected (impl in imp/unprotected.rs)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A marked pointer to a value loaded from an [`Atomic`] that is not protected
/// from reclamation and can hence not be safely de-referenced in general.
///
/// This type does have slightly stronger guarantees than a raw
/// [`MarkedPtr`][conquer_pointer::MarkedPtr], however, in that it must be
/// loaded from an [`Atomic`].
/// Consequently, as long as it is non-`null` and created by safe means, an
/// `Unprotected` is guaranteed to point at an (at least) once valid instance of
/// type `T`.
#[derive(Eq, Ord, PartialEq, PartialOrd)]
pub struct Unprotected<T, R, N> {
    inner: MarkedPtr<T, N>,
    _marker: PhantomData<R>,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// NotEqualError
////////////////////////////////////////////////////////////////////////////////////////////////////

/// An error type for indicating that a [`load_if_equal`][Atomic::load_if_equal]
/// operation failed because the loaded value did not match the expected one.
#[derive(Debug, Default, Copy, Clone, Hash, Eq, Ord, PartialEq, PartialOrd)]
pub struct NotEqualError;

/********** public functions **********************************************************************/

/// Returns a `null` pointer for an arbitrary type, [`Reclaim`] mechanism and
/// number of tag bits.
///
/// This function represents an ergonomic and handy way to generate a `null`
/// argument for eg. a [`store`][store] or [`compare_exchange`][cex] operation.
///
/// [store]: Atomic::store
/// [cex]: Atomic::compare_exchange
#[inline(always)]
pub const fn null<T, R, N>() -> Unprotected<T, R, N> {
    Unprotected::null()
}

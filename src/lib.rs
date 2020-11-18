//! TODO: crate lvl docs...

#![feature(min_const_generics, set_ptr_value)]
#![cfg_attr(not(any(test, feature = "std")), no_std)]
// #![warn(missing_docs)] todo: re-enable

extern crate alloc;

#[macro_use]
mod macros;

#[macro_use]
pub mod erased;
#[cfg(feature = "examples")]
pub mod examples;
pub mod fused;
pub mod leak;

mod alias;
mod atomic;
mod imp;
mod record;
mod retired;
#[macro_use]
mod traits;

use core::marker::PhantomData;

use conquer_pointer::{MarkedNonNull, MarkedPtr};

// public re-exports
pub use conquer_pointer;

pub use crate::atomic::{Atomic, Comparable, CompareExchangeErr, Storable};
pub use crate::retired::Retired;
pub use crate::traits::{
    Protect, ProtectExt, Reclaim, ReclaimBase, ReclaimRef, ReclaimThreadState,
};

// *************************************************************************************************
// Maybe
// *************************************************************************************************

/// An [`Option`]-like wrapper for non-nullable marked pointer or
/// reference types that can also represent marked `null` pointers.
#[derive(Copy, Clone, Debug, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub enum Maybe<P> {
    Some(P),
    Null(usize),
}

// *************************************************************************************************
// Owned (impl in imp/owned.rs)
// *************************************************************************************************

/// A smart pointer type for heap allocation similar to
/// [`Box`][alloc::boxed::Box].
///
/// Unlike [`Box`], the `Owned` type supports pointer tagging and is bound to
/// its associated [`Reclaim`] type.
/// The type guarantees that, on allocation, the instance of `T` will be
/// preceded by a [`Default`] initialized instance of the associated
/// [`Header`][ReclaimBase::Header] type.
///
/// When an [`Owned`] instance goes out scope, the entire record will be
/// de-allocated.
#[derive(Eq, Ord, PartialEq, PartialOrd)]
pub struct Owned<T, R: Reclaim<T>, const N: usize> {
    inner: MarkedNonNull<T, N>,
    _marker: PhantomData<(T, R)>,
}

// *************************************************************************************************
// Protected (impl in imp/protected.rs)
// *************************************************************************************************

/// A nullable shared pointer to a protected value that allows storing an
/// additional pointer tag.
#[derive(Eq, Ord, PartialEq, PartialOrd)]
pub struct Protected<'g, T, R, const N: usize> {
    inner: MarkedPtr<T, N>,
    _marker: PhantomData<(Option<&'g T>, R)>,
}

// *************************************************************************************************
// Shared (impl in imp/shared.rs)
// *************************************************************************************************

/// A local shared reference to a protected value that allows storing an
/// additional pointer tag.
///
/// Instances of `Shared` are derived from guard types (see [`Protect`]) from
/// which they inherit their lifetime dependence.
/// Like regular shared references (`&'g T`) they can be trivially copied,
/// cloned, de-referenced and can not be `null`.
///
/// See the documentation for [`deref`][Shared::as_ref] for an explanation of the
/// safety concerns involved in de-referencing a `Shared`.
#[derive(Eq, Ord, PartialEq, PartialOrd)]
pub struct Shared<'g, T, R, const N: usize> {
    inner: MarkedNonNull<T, N>,
    _marker: PhantomData<(&'g T, R)>,
}

// *************************************************************************************************
// Unlinked (impl in imp/unlinked.rs)
// *************************************************************************************************

/// A reference type for a value that has been removed from its previous
/// location as the result of a [`swap`][swap] or [`compare_exchange`][cex]
/// operation on an [`Atomic`].
///
/// Under the assumption that no other thread is able to load a *new* reference
/// to the same value as the `Unlinked` after it being unlinked, it is sound to
/// [`into_retired`][Unlinked::into_retired] the value, which hands it over to
/// the [`Reclaim`] mechanism for eventual de-allocation.
/// This is for instance always the case if the [`Atomic`] was a unique pointer
/// to the unlinked value.
///
/// [swap]: Atomic::swap
/// [cex]: Atomic::compare_exchange
#[derive(Eq, Ord, PartialEq, PartialOrd)]
#[must_use = "unlinked values are meant to be retired, otherwise a memory leak is highly likely"]
pub struct Unlinked<T, R, const N: usize> {
    inner: MarkedNonNull<T, N>,
    _marker: PhantomData<(T, R)>,
}

// *************************************************************************************************
// Unprotected (impl in imp/unprotected.rs)
// *************************************************************************************************

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
pub struct Unprotected<T, R, const N: usize> {
    inner: MarkedPtr<T, N>,
    _marker: PhantomData<R>,
}

// *************************************************************************************************
// NotEqual
// *************************************************************************************************

/// A type for indicating that a [`load_if_equal`][Atomic::load_if_equal]
/// operation failed due to the actual value not matching the expected one.
#[derive(Debug, Default, Copy, Clone, Hash, Eq, Ord, PartialEq, PartialOrd)]
pub struct NotEqual;

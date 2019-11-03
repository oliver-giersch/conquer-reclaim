mod imp;

use core::ops::Deref;
use core::sync::atomic::Ordering;

use conquer_pointer::{MarkedNonNull, MarkedNonNullable, MarkedOption, MarkedPtr};
use typenum::Unsigned;

use crate::atomic::Atomic;
use crate::internal::Internal;
use crate::retired::Retired;
use crate::Shared;

////////////////////////////////////////////////////////////////////////////////////////////////////
// GlobalReclaim (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: Docs...
pub unsafe trait GlobalReclaim: Reclaim {
    /// TODO: Docs...
    type Guard: Protect<Reclaimer = Self> + Default + 'static;

    /// TODO: Docs...
    fn guard() -> Self::Guard {
        Self::Guard::default()
    }

    /// TODO: Docs...
    unsafe fn retire_global(record: Retired<Self>);
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Reclaim (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: Docs...
pub unsafe trait Reclaim: Sized + 'static {
    /// TODO: Docs...
    type Handle: ReclaimHandle<Reclaimer = Self>;
    /// TODO: Docs...
    type Header: Default + Sync + Sized;
    /// TODO: Docs...
    type Global: Default + Sync + Sized;

    /// TODO: Docs...
    fn create_handle(global: impl Deref<Target = Self::Global>) -> Self::Handle;
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ReclaimHandle (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: Docs...
pub trait ReclaimHandle: Clone + Sized {
    /// TODO: Docs...
    type Reclaimer: Reclaim;
    /// TODO: Docs...
    type Guard: Protect<Reclaimer = Self::Reclaimer>;

    /// TODO: Docs...
    fn guard(&self) -> Self::Guard;
    /// TODO: Docs...
    unsafe fn retire(&self, record: Retired<Self::Reclaimer>);
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Protect (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: Docs...
pub unsafe trait Protect: Clone + Sized {
    /// The associated memory reclamation scheme.    
    type Reclaimer: Reclaim;

    /// Releases any protection that may be provided by the guard.
    ///
    /// This method has no effect for guards that also implement
    /// [`ProtectRegion`].
    fn release(&mut self);

    /// TODO: Docs...
    fn protect<T, N: Unsigned>(
        &mut self,
        src: &Atomic<T, Self::Reclaimer, N>,
        order: Ordering,
    ) -> MarkedOption<Shared<T, Self::Reclaimer, N>>;

    /// TODO: Docs...
    fn protect_if_equal<T, N: Unsigned>(
        &mut self,
        src: &Atomic<T, Self::Reclaimer, N>,
        expected: MarkedPtr<T, N>,
        order: Ordering,
    ) -> crate::AcquireResult<T, Self::Reclaimer, N>;
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ProtectRegion (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: Docs...
pub unsafe trait ProtectRegion: Protect {}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ReclaimPointer (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait SharedPointer: Sized + Internal {
    /// The pointed-to type.
    type Item: Sized;
    /// TODO: Docs...
    type Reclaimer: Reclaim;
    /// Number of bits available for tagging.
    type MarkBits: Unsigned;
    /// TODO: Docs...
    type Pointer: MarkedNonNullable<Item = Self::Item, MarkBits = Self::MarkBits>;

    /// TODO: Docs
    fn with(ptr: Self::Pointer) -> Self;

    /// TODO: Docs... (necessary method?)
    fn compose(ptr: Self::Pointer, tag: usize) -> Self;

    /// TODO: Docs...
    unsafe fn from_marked_ptr(marked_ptr: MarkedPtr<Self::Item, Self::MarkBits>) -> Self;

    /// TODO: Docs...
    unsafe fn from_marked_non_null(marked_ptr: MarkedNonNull<Self::Item, Self::MarkBits>) -> Self;

    /// TODO: Docs...
    fn as_marked_ptr(&self) -> MarkedPtr<Self::Item, Self::MarkBits>;

    /// TODO: Docs...
    fn into_marked_ptr(self) -> MarkedPtr<Self::Item, Self::MarkBits>;

    /// TODO: Docs...
    fn clear_tag(self) -> Self;

    /// TODO: Docs...
    fn set_tag(self, tag: usize) -> Self;

    /// TODO: Docs... (necessary method?)
    fn decompose(self) -> (Self, usize);
}

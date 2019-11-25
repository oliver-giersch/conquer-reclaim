mod imp;

use core::sync::atomic::Ordering;

use conquer_pointer::{MarkedNonNull, MarkedNonNullable, MarkedOption, MarkedPtr};
use typenum::Unsigned;

use crate::atomic::Atomic;
use crate::internal::Internal;
use crate::retired::Retired;
use crate::Shared;

////////////////////////////////////////////////////////////////////////////////////////////////////
// GlobalReclaimer (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: Docs...
pub unsafe trait GlobalReclaimer: Reclaimer {
    /// TODO: Docs...
    fn guard() -> <Self::Handle as ReclaimerHandle>::Guard;
    /// TODO: Docs...
    unsafe fn retire(record: Retired<Self>);
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Reclaimer (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: Docs...
pub unsafe trait Reclaimer: Default + Sync + Sized + 'static {
    /// TODO: docs...
    type Global: Default + Sync + Sized;
    /// TODO: docs...
    type Header: Default + Sync + Sized + 'static;
    /// TODO: docs...
    type Handle: ReclaimerHandle<Reclaimer = Self>;

    /// TODO: docs...
    fn create_local_handle(&self) -> Self::Handle;
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ReclaimerHandle (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: Docs...
pub unsafe trait ReclaimerHandle: Clone + Sized {
    /// TODO: Docs...
    type Reclaimer: Reclaimer;
    /// TODO: Docs...
    type Guard: Protect<Reclaimer = Self::Reclaimer>;

    /// TODO: Docs...
    fn guard(self) -> Self::Guard;
    /// TODO: Docs...
    unsafe fn retire(self, record: Retired<Self::Reclaimer>);
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Protect (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: Docs...
pub unsafe trait Protect: Clone + Sized {
    /// The associated memory reclaimer.
    type Reclaimer: Reclaimer;

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

/// TODO: docs...
pub trait SharedPointer: Sized + Internal {
    /// The pointed-to type.
    type Item: Sized;
    /// TODO: Docs...
    type Reclaimer: Reclaimer;
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

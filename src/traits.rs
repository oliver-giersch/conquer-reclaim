use core::sync::atomic::Ordering;

use conquer_pointer::{MarkedPtr, MaybeNull};
use typenum::Unsigned;

use crate::atomic::Atomic;
use crate::retired::Retired;
use crate::Shared;

////////////////////////////////////////////////////////////////////////////////////////////////////
// GlobalReclaimer (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: Docs...
pub unsafe trait GlobalReclaimer: GenericReclaimer {
    /// TODO: Docs...
    fn guard() -> <Self::Handle as ReclaimerHandle>::Guard;
    /// TODO: Docs...
    unsafe fn retire(record: Retired<Self>);
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// GenericReclaimer (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: docs...
pub unsafe trait GenericReclaimer: Reclaimer {
    /// TODO: docs...
    type Handle: ReclaimerHandle<Reclaimer = Self>;

    /// TODO: docs...
    fn create_local_handle(&self) -> Self::Handle;
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
    ) -> MaybeNull<Shared<T, Self::Reclaimer, N>>;

    /// TODO: Docs...
    fn protect_if_equal<T, N: Unsigned>(
        &mut self,
        src: &Atomic<T, Self::Reclaimer, N>,
        expected: MarkedPtr<T, N>,
        order: Ordering,
    ) -> Result<MaybeNull<Shared<T, Self::Reclaimer, N>>, crate::NotEqualError>;
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ProtectRegion (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: Docs...
pub unsafe trait ProtectRegion: Protect {}

use typenum::Unsigned;

use crate::traits::{Reclaimer, SharedPointer};
use crate::{Shared, Unlinked};
use conquer_pointer::MarkedPtr;

////////////////////////////////////////////////////////////////////////////////////////////////////
// CompareArg (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait CompareArg {
    type Item: Sized;
    type Reclaimer: Reclaimer;
    type MarkBits: Unsigned;

    fn into_marked_ptr(arg: Self) -> MarkedPtr<Self::Item, Self::MarkBits>;
    unsafe fn from_marked_ptr(ptr: MarkedPtr<Self::Item, Self::MarkBits>) -> Self;
}

/// TODO: Docs...
pub trait CompareAndSwap: SharedPointer {
    /// TODO: Docs...
    type Success: SharedPointer<
        Item = Self::Item,
        Reclaimer = Self::Reclaimer,
        MarkBits = Self::MarkBits,
        Pointer = Unlinked<Self::Item, Self::Reclaimer, Self::MarkBits>,
    >;
}

impl<'g, T, R: Reclaimer, N: Unsigned> CompareAndSwap for Shared<'g, T, R, N> {
    type Success = Unlinked<T, R, N>;
}

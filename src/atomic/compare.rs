use crate::traits::SharedPointer;
use crate::{Unlinked, Unprotected};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Compare (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: Docs...
pub trait CompareAndSwap: SharedPointer {
    /// TODO: Docs...
    type Success: SharedPointer<
        Item = Self::Item,
        Reclaimer = Self::Reclaimer,
        MarkBits = Self::MarkBits,
        Pointer = Unlinked<Self::Item, Self::Reclaimer, Self::MarkBits>,
    >;

    /// TODO: Docs...
    type Failure: SharedPointer<
        Item = Self::Item,
        Reclaimer = Self::Reclaimer,
        MarkBits = Self::MarkBits,
        Pointer = Unprotected<Self::Item, Self::Reclaimer, Self::MarkBits>,
    >;
}

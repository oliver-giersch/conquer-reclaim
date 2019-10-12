use crate::traits::ReclaimPointer;
use crate::{Unlinked, Unprotected};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Compare (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: Docs...
pub trait CompareAndSwap: ReclaimPointer {
    /// TODO: Docs...
    type Success: ReclaimPointer<
        Item = Self::Item,
        Reclaimer = Self::Reclaimer,
        MarkBits = Self::MarkBits,
        Pointer = Unlinked<Self::Item, Self::Reclaimer, Self::MarkBits>,
    >;

    /// TODO: Docs...
    type Failure: ReclaimPointer<
        Item = Self::Item,
        Reclaimer = Self::Reclaimer,
        MarkBits = Self::MarkBits,
        Pointer = Unprotected<Self::Item, Self::Reclaimer, Self::MarkBits>,
    >;
}

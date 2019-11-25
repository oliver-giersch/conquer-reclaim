use typenum::Unsigned;

use crate::traits::{Reclaimer, SharedPointer};
use crate::{Shared, Unlinked};

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
}

impl<'g, T, R: Reclaimer, N: Unsigned> CompareAndSwap for Shared<'g, T, R, N> {
    type Success = Unlinked<T, R, N>;
}

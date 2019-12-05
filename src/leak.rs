//! TODO: mod-level docs

use core::marker::PhantomData;
use core::sync::atomic::Ordering;

use conquer_pointer::{
    MarkedPtr,
    MaybeNull::{self, NotNull},
};
use typenum::Unsigned;

use crate::retired::Retired;
use crate::traits::{
    GenericReclaimer, GlobalReclaimer, Protect, ProtectRegion, Reclaimer, ReclaimerHandle,
};
use crate::NotEqualError;

/// TODO: docs...
pub type Atomic<T, N> = crate::atomic::Atomic<T, Leaking, N>;
/// TODO: docs...
pub type Owned<T, N> = crate::Owned<T, Leaking, N>;
/// TODO: docs...
pub type Shared<'g, T, N> = crate::Shared<'g, T, Leaking, N>;
/// TODO: docs...
pub type Unlinked<T, N> = crate::Unlinked<T, Leaking, N>;
/// TODO: docs...
pub type Unprotected<T, N> = crate::Unprotected<T, Leaking, N>;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Leaking
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: docs...
#[derive(Copy, Clone, Debug, Default, Hash, Eq, Ord, PartialEq, PartialOrd)]
pub struct Leaking;

/********** impl GlobalReclaim ********************************************************************/

unsafe impl GlobalReclaimer for Leaking {
    #[inline]
    fn guard() -> <Self::Handle as ReclaimerHandle>::Guard {
        Guard
    }

    #[inline]
    unsafe fn retire(_: Retired<Self>) {}
}

/********** impl GenericReclaimer *****************************************************************/

unsafe impl GenericReclaimer for Leaking {
    type Handle = Handle;

    #[inline]
    fn create_local_handle(&self) -> Self::Handle {
        Handle
    }
}

/********** impl Reclaimer ************************************************************************/

unsafe impl Reclaimer for Leaking {
    type Global = ();
    type Header = ();
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Handle
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: docs...
#[derive(Copy, Clone, Default, Debug)]
pub struct Handle;

/********** impl ReclaimHandle ********************************************************************/

unsafe impl ReclaimerHandle for Handle {
    type Reclaimer = Leaking;
    type Guard = Guard;

    #[inline]
    fn guard(self) -> Self::Guard {
        Guard
    }

    #[inline]
    unsafe fn retire(self, _: Retired<Self::Reclaimer>) {}
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Guard
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: docs...
#[derive(Debug, Default, Copy, Clone, Eq, Ord, PartialEq, PartialOrd)]
pub struct Guard;

/********** impl Protect **************************************************************************/

unsafe impl Protect for Guard {
    type Reclaimer = Leaking;

    #[inline]
    fn release(&mut self) {}

    #[inline]
    fn protect<T, N: Unsigned>(
        &mut self,
        atomic: &Atomic<T, N>,
        order: Ordering,
    ) -> MaybeNull<Shared<T, N>> {
        MaybeNull::from(atomic.load_raw(order))
            .map(|ptr| Shared { inner: ptr, _marker: PhantomData })
    }

    #[inline]
    fn protect_if_equal<T, N: Unsigned>(
        &mut self,
        atomic: &Atomic<T, N>,
        expected: MarkedPtr<T, N>,
        order: Ordering,
    ) -> Result<MaybeNull<Shared<T, N>>, NotEqualError> {
        match MaybeNull::from(atomic.load_raw(order)) {
            MaybeNull::NotNull(ptr) if ptr.into_marked_ptr() == expected => {
                Ok(NotNull(Shared { inner: ptr, _marker: PhantomData }))
            }
            _ => Err(NotEqualError(())),
        }
    }
}

/********** impl ProtectRegion ********************************************************************/

unsafe impl ProtectRegion for Guard {}

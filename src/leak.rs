//! TODO: mod-level docs

use core::marker::PhantomData;
use core::ops::Deref;
use core::sync::atomic::Ordering;

use conquer_pointer::{MarkedOption, MarkedPtr};
use typenum::Unsigned;

use crate::retired::Retired;
use crate::traits::{GlobalReclaim, Protect, ProtectRegion, Reclaim, ReclaimHandle};
use crate::AcquireResult;

pub type Atomic<T, N> = crate::atomic::Atomic<T, Leaking, N>;
pub type Owned<T, N> = crate::Owned<T, Leaking, N>;
pub type Shared<'g, T, N> = crate::Shared<'g, T, Leaking, N>;
pub type Unlinked<T, N> = crate::Unlinked<T, Leaking, N>;
pub type Unprotected<T, N> = crate::Unprotected<T, Leaking, N>;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Leaking
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Copy, Clone, Debug, Default, Hash, Eq, Ord, PartialEq, PartialOrd)]
pub struct Leaking;

/********** impl GlobalReclaim ********************************************************************/

unsafe impl GlobalReclaim for Leaking {
    type Guard = Guard;

    #[inline]
    unsafe fn retire_global(_: Retired<Self>) {}
}

/********** impl Reclaim **************************************************************************/

unsafe impl Reclaim for Leaking {
    type DefaultHandle = Handle;
    type Header = ();
    type Global = ();
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Handle
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Default, Clone)]
pub struct Handle;

/********** impl ReclaimHandle ********************************************************************/

impl ReclaimHandle for Handle {
    type Reclaimer = Leaking;
    type GlobalHandle = &'static ();
    type Guard = Guard;

    #[inline]
    fn new(_: Self::GlobalHandle) -> Self {
        Self
    }

    #[inline]
    fn guard(&self) -> Self::Guard {
        Guard
    }

    #[inline]
    unsafe fn retire(&self, _: Retired<Self::Reclaimer>) {}
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Guard
////////////////////////////////////////////////////////////////////////////////////////////////////

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
        src: &Atomic<T, N>,
        order: Ordering,
    ) -> MarkedOption<Shared<T, N>> {
        MarkedOption::from(src.load_raw(order))
            .map(|ptr| Shared { inner: ptr, _marker: PhantomData })
    }

    #[inline]
    fn protect_if_equal<T, N: Unsigned>(
        &mut self,
        src: &Atomic<T, N>,
        expected: MarkedPtr<T, N>,
        order: Ordering,
    ) -> AcquireResult<T, Self::Reclaimer, N> {
        match src.load_raw(order) {
            ptr if ptr == expected => {
                Ok(MarkedOption::from(ptr).map(|ptr| Shared { inner: ptr, _marker: PhantomData }))
            }
            _ => Err(crate::NotEqualError(())),
        }
    }
}

/********** impl ProtectRegion ********************************************************************/

unsafe impl ProtectRegion for Guard {}

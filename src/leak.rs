use conquer_pointer::MarkedOption::Value;
use typenum::Unsigned;

use crate::retired::Retired;
use crate::traits::{Protect, ProtectRegion, Reclaim, ReclaimHandle};
use crate::AcquireResult;

use conquer_pointer::{MarkedOption, MarkedPtr};
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::atomic::Ordering;

pub type Atomic<T, N> = crate::atomic::Atomic<T, Leaking, N>;
pub type Shared<'g, T, N> = crate::Shared<'g, T, Leaking, N>;

pub struct Leaking;

#[derive(Default, Clone)]
pub struct Handle;

impl ReclaimHandle for Handle {
    type Reclaimer = Leaking;
    type Guard = Guard;

    fn guard(&self) -> Self::Guard {
        unimplemented!()
    }

    unsafe fn retire(&self, record: Retired<Self::Reclaimer>) {
        unimplemented!()
    }
}

unsafe impl Reclaim for Leaking {
    type DefaultHandle = Handle;
    type Header = ();
    type Global = ();

    #[inline]
    fn create_handle(_: impl Deref<Target = Self::Global>) -> Self::DefaultHandle {
        Handle
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Guard
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Default, Copy, Clone, Eq, Ord, PartialEq, PartialOrd)]
pub struct Guard;

/********** impl inherent *************************************************************************/

impl Guard {
    #[inline]
    pub const fn new() -> Self {
        Self
    }
}

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
            ptr if ptr == expected => unimplemented!(),
            _ => Err(crate::NotEqualError(())),
        }
    }
}

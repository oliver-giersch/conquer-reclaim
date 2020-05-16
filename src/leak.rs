//! TODO: mod-level docs

use core::marker::PhantomData;
use core::sync::atomic::Ordering;

use conquer_pointer::typenum::Unsigned;
use conquer_pointer::MarkedPtr;

use crate::reclaim::Leaking;
use crate::retired::Retired;
use crate::traits::{Protect, ReclaimLocalState, ReclaimRef, Retire};
use crate::NotEqual;

/// A specialization of the [`Atomic`](crate::atomic::Atomic) type using
/// [`Leaking`] as reclaimer.
pub type Atomic<T, N> = crate::atomic::Atomic<T, Leaking, N>;
/// A specialization of the [`Owned`](crate::Owned) type using [`Leaking`] as
/// reclaimer.
pub type Owned<T, N> = crate::Owned<T, Leaking, N>;
/// A specialization of the [`Protected`](crate::Protected) type using
/// [`Leaking`] as reclaimer.
pub type Protected<'g, T, N> = crate::Protected<'g, T, Leaking, N>;
/// A specialization of the [`Shared`](crate::Shared) type using [`Leaking`] as
/// reclaimer.
pub type Shared<'g, T, N> = crate::Shared<'g, T, Leaking, N>;
/// A specialization of the [`Unlinked`](crate::Unlinked) type using [`Leaking`]
/// as reclaimer.
pub type Unlinked<T, N> = crate::Unlinked<T, Leaking, N>;
/// A specialization of the [`Unprotected`](crate::Unprotected) type using
/// [`Leaking`] as reclaimer.
pub type Unprotected<T, N> = crate::Unprotected<T, Leaking, N>;

////////////////////////////////////////////////////////////////////////////////////////////////////
// LeakingRef
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A no-op [`GlobalReclaimer`] that deliberately leaks memory.
//#[derive(Debug, Default, Hash, Eq, Ord, PartialEq, PartialOrd)]
pub struct LeakingRef;

/********** impl ReclaimRef ***********************************************************************/

impl ReclaimRef for LeakingRef {
    type LocalState = LeakingLocalState;
    type Reclaim = Leaking;

    #[inline(always)]
    unsafe fn build_local_state(&self) -> Self::LocalState {
        LeakingLocalState
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// LeakingLocalState
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Debug, Default)]
pub struct LeakingLocalState;

/********** impl LocalState ***********************************************************************/

impl ReclaimLocalState for LeakingLocalState {
    type Guard = Guard;
    type Reclaim = Leaking;

    #[inline(always)]
    fn build_guard(&self) -> Self::Guard {
        Guard
    }

    #[inline(always)]
    unsafe fn retire_record(&self, _: Retired<Self::Reclaim>) {}
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Guard
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Copy, Clone, Debug, Default)]
pub struct Guard;

macro_rules! impl_protect {
    () => {
        type Reclaim = Leaking;

        #[inline]
        fn protect<T, N: Unsigned>(
            &mut self,
            atomic: &Atomic<T, N>,
            order: Ordering,
        ) -> Protected<T, N>
        where
            Self::Reclaim: Retire<T>,
        {
            Protected { inner: atomic.load_raw(order), _marker: PhantomData }
        }

        #[inline]
        fn protect_if_equal<T, N: Unsigned>(
            &mut self,
            atomic: &Atomic<T, N>,
            expected: MarkedPtr<T, N>,
            order: Ordering,
        ) -> Result<Protected<T, N>, NotEqual>
        where
            Self::Reclaim: Retire<T>,
        {
            atomic
                .load_raw_if_equal(expected, order)
                .map(|inner| Protected { inner, _marker: PhantomData })
        }
    };
}

/********** impl Protect (Guard) ******************************************************************/

unsafe impl Protect for Guard {
    impl_protect!();
}

/********** impl Protect (&Guard) *****************************************************************/

unsafe impl Protect for &Guard {
    impl_protect!();
}

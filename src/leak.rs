//! TODO: mod-level docs

use core::marker::PhantomData;
use core::sync::atomic::Ordering;

use conquer_pointer::typenum::Unsigned;
use conquer_pointer::MarkedPtr;

use crate::reclaim::Typed;
use crate::retired::Retired;
use crate::traits::{Protect, ReclaimLocalState, ReclaimRef};
use crate::NotEqual;

/// A specialization of the [`Atomic`](crate::atomic::Atomic) type using
/// [`Leaking`] as reclaimer.
pub type Atomic<T, N> = crate::atomic::Atomic<T, LeakReclaim<T>, N>;
/// A specialization of the [`Owned`](crate::Owned) type using [`Leaking`] as
/// reclaimer.
pub type Owned<T, N> = crate::Owned<T, LeakReclaim<T>, N>;
/// A specialization of the [`Protected`](crate::Protected) type using
/// [`Leaking`] as reclaimer.
pub type Protected<'g, T, N> = crate::Protected<'g, T, LeakReclaim<T>, N>;
/// A specialization of the [`Shared`](crate::Shared) type using [`Leaking`] as
/// reclaimer.
pub type Shared<'g, T, N> = crate::Shared<'g, T, LeakReclaim<T>, N>;
/// A specialization of the [`Unlinked`](crate::Unlinked) type using [`Leaking`]
/// as reclaimer.
pub type Unlinked<T, N> = crate::Unlinked<T, LeakReclaim<T>, N>;
/// A specialization of the [`Unprotected`](crate::Unprotected) type using
/// [`Leaking`] as reclaimer.
pub type Unprotected<T, N> = crate::Unprotected<T, LeakReclaim<T>, N>;

////////////////////////////////////////////////////////////////////////////////////////////////////
// LeakReclaim (type alias)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub type LeakReclaim<T> = Typed<T, ()>;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Leaking
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A no-op [`GlobalReclaimer`] that deliberately leaks memory.
//#[derive(Debug, Default, Hash, Eq, Ord, PartialEq, PartialOrd)]
pub struct Leaking<T>(LeakReclaim<T>);

/********** impl Clone ****************************************************************************/

impl<T> Clone for Leaking<T> {
    #[inline]
    fn clone(&self) -> Self {
        Leaking(Default::default())
    }
}

/********** impl ReclaimRef ***********************************************************************/

impl<T> ReclaimRef<T> for Leaking<T> {
    type LocalState = LeakingLocalState<T>;
    type Reclaim = LeakReclaim<T>;

    #[inline]
    unsafe fn build_local_state(&self) -> Self::LocalState {
        LeakingLocalState(Default::default())
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// LeakingLocalState
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Debug, Default)]
pub struct LeakingLocalState<T>(LeakReclaim<T>);

/********** impl LocalState ***********************************************************************/

impl<T> ReclaimLocalState<T> for LeakingLocalState<T> {
    type Guard = Guard<T>;
    type Reclaim = LeakReclaim<T>;

    #[inline]
    fn build_guard(&self) -> Self::Guard {
        Guard(Default::default())
    }

    #[inline]
    unsafe fn retire_record(&self, _: Retired<Self::Reclaim>) {}
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Guard
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Default)]
pub struct Guard<T>(LeakReclaim<T>);

impl<T> Clone for Guard<T> {
    fn clone(&self) -> Self {
        Guard(Default::default())
    }
}

macro_rules! impl_protect {
    () => {
        type Reclaim = LeakReclaim<T>;

        #[inline]
        fn protect<N: Unsigned>(
            &mut self,
            atomic: &Atomic<T, N>,
            order: Ordering,
        ) -> Protected<T, N> {
            Protected { inner: atomic.load_raw(order), _marker: PhantomData }
        }

        #[inline]
        fn protect_if_equal<N: Unsigned>(
            &mut self,
            atomic: &Atomic<T, N>,
            expected: MarkedPtr<T, N>,
            order: Ordering,
        ) -> Result<Protected<T, N>, NotEqual> {
            atomic
                .load_raw_if_equal(expected, order)
                .map(|inner| Protected { inner, _marker: PhantomData })
        }
    };
}

/********** impl Protect (Guard) ******************************************************************/

unsafe impl<T> Protect<T> for Guard<T> {
    impl_protect!();
}

/********** impl Protect (&Guard) *****************************************************************/

unsafe impl<T> Protect<T> for &Guard<T> {
    impl_protect!();
}

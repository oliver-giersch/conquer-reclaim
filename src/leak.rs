//! TODO: mod-level docs

use core::marker::PhantomData;
use core::sync::atomic::Ordering;

use conquer_pointer::typenum::Unsigned;
use conquer_pointer::MarkedPtr;

use crate::retired::Retired;
use crate::traits::{GlobalReclaim, LocalState, Protect, Reclaim};
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
// Leaking
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A no-op [`GlobalReclaimer`] that deliberately leaks memory.
#[derive(Copy, Clone, Debug, Default, Hash, Eq, Ord, PartialEq, PartialOrd)]
pub struct Leaking;

/********** impl GlobalReclaim ********************************************************************/

unsafe impl GlobalReclaim for Leaking {
    #[inline]
    fn build_local_state() -> Self::LocalState {
        LeakingLocalState
    }
}

/********** impl Reclaim **************************************************************************/

impl Reclaim for Leaking {
    type Header = ();
    type LocalState = LeakingLocalState;

    #[inline]
    unsafe fn build_local_state(&self) -> Self::LocalState {
        LeakingLocalState
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// LeakingLocalState
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Copy, Debug, Default)]
pub struct LeakingLocalState;

/********** impl LocalState ***********************************************************************/

unsafe impl LocalState for LeakingLocalState {
    type Guard = Guard;
    type Reclaimer = Leaking;

    #[inline]
    fn build_guard(&self) -> Self::Guard {
        Guard
    }

    #[inline]
    unsafe fn retire_record(&self, _: Retired<Self::Reclaimer>) {}
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Guard
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Copy, Clone, Debug, Default)]
pub struct Guard;

macro_rules! impl_protect {
    () => {
        type Reclaimer = Leaking;

        #[inline]
        fn protect<T, N: Unsigned>(
            &mut self,
            atomic: &Atomic<T, N>,
            order: Ordering,
        ) -> Protected<T, N> {
            Protected { inner: atomic.load_raw(order), _marker: PhantomData }
        }

        #[inline]
        fn protect_if_equal<T, N: Unsigned>(
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

unsafe impl Protect for Guard {
    impl_protect!();
}

/********** impl Protect (&Guard) *****************************************************************/

unsafe impl Protect for &Guard {
    impl_protect!();
}

//! TODO: mod-level docs

use core::marker::PhantomData;
use core::sync::atomic::Ordering;

use conquer_pointer::typenum::Unsigned;
use conquer_pointer::MarkedPtr;

use crate::retired::Retired;
use crate::traits::{Protect, Reclaim, ReclaimBase, ReclaimRef, ReclaimThreadState};
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

#[derive(Debug, Default, Hash, Eq, Ord, PartialEq, PartialOrd)]
pub struct Leaking;

/********** impl ReclaimBase **********************************************************************/

unsafe impl ReclaimBase for Leaking {
    type Header = ();
    type Retired = ();
}

/********** impl Reclaim **************************************************************************/

unsafe impl<T> Reclaim<T> for Leaking {
    #[inline(always)]
    unsafe fn retire(ptr: *mut T) -> *mut () {
        ptr.cast()
    }
}

/********** impl ReclaimRef ***********************************************************************/

unsafe impl<T> ReclaimRef<T> for Leaking {
    type Reclaim = Self;
    type ThreadState = Self;

    #[inline(always)]
    fn alloc_owned<N: Unsigned>(&self, value: T) -> Owned<T, N> {
        Owned::new(value)
    }

    #[inline(always)]
    unsafe fn build_thread_state_unchecked(&self) -> Self::ThreadState {
        Leaking
    }
}

/********** impl ReclaimThreadState ***************************************************************/

unsafe impl<T> ReclaimThreadState<T> for Leaking {
    type Reclaim = Self;
    type Guard = Guard;

    #[inline(always)]
    fn build_guard(&self) -> Self::Guard {
        Guard
    }

    #[inline(always)]
    fn alloc_owned<N: Unsigned>(&self, value: T) -> Owned<T, N> {
        Owned::new(value)
    }

    #[inline(always)]
    unsafe fn retire_record(&self, _: Retired<Leaking>) {}
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Guard
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Copy, Debug, Default, Hash, Eq, Ord, PartialEq, PartialOrd)]
pub struct Guard;

macro_rules! impl_protect {
    () => {
        type Reclaim = Leaking;

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

unsafe impl<T> Protect<T> for Guard {
    impl_protect!();
}

/********** impl Protect (&Guard) *****************************************************************/

unsafe impl<T> Protect<T> for &Guard {
    impl_protect!();
}

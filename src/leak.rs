//! TODO: mod-level docs

use core::marker::PhantomData;
use core::sync::atomic::Ordering;

use conquer_pointer::typenum::Unsigned;
use conquer_pointer::MarkedPtr;

use crate::traits::{Protect, Reclaim, ReclaimLocalState, ReclaimRef, Retire};
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

/********** impl Reclaim **************************************************************************/

unsafe impl Reclaim for Leaking {
    type Header = ();
    type Retired = ();

    #[inline(always)]
    unsafe fn reclaim(_: *mut Self::Header) {}
}

/********** impl Retire ***************************************************************************/

unsafe impl<T> Retire<T> for Leaking {
    #[inline(always)]
    unsafe fn retire(ptr: *mut T) -> *mut Self::Retired {
        ptr.cast()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// LeakingRef
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A no-op [`GlobalReclaimer`] that deliberately leaks memory.
//#[derive(Debug, Default, Hash, Eq, Ord, PartialEq, PartialOrd)]
pub struct LeakingRef<T>(PhantomData<T>);

/********** impl ReclaimRef ***********************************************************************/

unsafe impl<T> ReclaimRef for LeakingRef<T> {
    type Item = T;
    type Reclaim = Leaking;
    type LocalState = LeakingLocalState<T>;

    #[inline]
    fn alloc_owned<N: Unsigned>(&self, value: Self::Item) -> Owned<Self::Item, N> {
        unsafe { Owned::with_header((), value) }
    }

    #[inline(always)]
    unsafe fn build_local_state(&self) -> Self::LocalState {
        LeakingLocalState(PhantomData)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// LeakingLocalState
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct LeakingLocalState<T>(PhantomData<T>);

/********** impl LocalState ***********************************************************************/

impl<T> ReclaimLocalState for LeakingLocalState<T> {
    type Item = T;
    type Reclaim = Leaking;

    type Guard = Guard<T>;

    #[inline]
    fn alloc_owned<N: Unsigned>(&self, value: Self::Item) -> Owned<Self::Item, N> {
        unsafe { Owned::with_header((), value) }
    }

    #[inline(always)]
    fn build_guard(&self) -> Self::Guard {
        Guard(PhantomData)
    }

    #[inline(always)]
    unsafe fn retire_record<N: Unsigned>(&self, _: Unlinked<T, N>) {}
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Guard
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Copy, Debug, Default)]
pub struct Guard<T>(PhantomData<T>);

/********** impl Clone ****************************************************************************/

impl<T> Clone for Guard<T> {
    #[inline]
    fn clone(&self) -> Self {
        Guard(PhantomData)
    }
}

macro_rules! impl_protect {
    () => {
        type Item = T;
        type Reclaim = Leaking;

        #[inline]
        fn protect<N: Unsigned>(
            &mut self,
            atomic: &Atomic<Self::Item, N>,
            order: Ordering,
        ) -> Protected<Self::Item, N> {
            Protected { inner: atomic.load_raw(order), _marker: PhantomData }
        }

        #[inline]
        fn protect_if_equal<N: Unsigned>(
            &mut self,
            atomic: &Atomic<Self::Item, N>,
            expected: MarkedPtr<Self::Item, N>,
            order: Ordering,
        ) -> Result<Protected<Self::Item, N>, NotEqual> {
            atomic
                .load_raw_if_equal(expected, order)
                .map(|inner| Protected { inner, _marker: PhantomData })
        }
    };
}

/********** impl Protect (Guard) ******************************************************************/

unsafe impl<T> Protect for Guard<T> {
    impl_protect!();
}

/********** impl Protect (&Guard) *****************************************************************/

unsafe impl<T> Protect for &Guard<T> {
    impl_protect!();
}

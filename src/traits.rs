use core::mem;
use core::ops::Deref;
use core::sync::atomic::Ordering;

#[cfg(not(feature = "std"))]
use alloc::boxed::Box;

use conquer_pointer::typenum::Unsigned;
use conquer_pointer::MarkedPtr;

use crate::alias::RetiredRecord;
use crate::atomic::Atomic;
use crate::fused::FusedGuardRef;
use crate::{Maybe, NotEqual, Owned, Protected, Retired, Shared};

////////////////////////////////////////////////////////////////////////////////////////////////////
// ReclaimBase (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub unsafe trait ReclaimBase: Sync + Sized {
    type Header: Sized;
    type Retired: ?Sized;

    #[inline]
    unsafe fn reclaim(retired: *mut Self::Retired) {
        let record = RetiredRecord::<Self>::record_from_data(retired);
        Box::from_raw(record);
    }

    #[inline]
    unsafe fn as_data_ptr(retired: *mut Self::Retired) -> *mut () {
        retired as *mut _
    }

    #[inline]
    unsafe fn as_header_ptr(retired: *mut Self::Retired) -> *mut Self::Header {
        RetiredRecord::<Self>::header_from_data(retired)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Reclaim (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub unsafe trait Reclaim<T>: ReclaimBase {
    unsafe fn retire(ptr: *mut T) -> *mut Self::Retired;
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ReclaimRef (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub unsafe trait ReclaimRef<T>: Sync + Sized {
    type Reclaim: Reclaim<T>;
    type ThreadState: ReclaimThreadState<T, Reclaim = Self::Reclaim>;

    fn alloc_owned<N: Unsigned>(&self, value: T) -> Owned<T, Self::Reclaim, N>;
    unsafe fn build_thread_state_unchecked(&self) -> Self::ThreadState;
}

/*********** blanket impl *************************************************************************/

unsafe impl<T, R: Sync + Deref> ReclaimRef<T> for R
where
    R::Target: ReclaimRef<T>,
{
    type Reclaim = <R::Target as ReclaimRef<T>>::Reclaim;
    type ThreadState = <R::Target as ReclaimRef<T>>::ThreadState;

    #[inline]
    fn alloc_owned<N: Unsigned>(&self, value: T) -> Owned<T, Self::Reclaim, N> {
        (**self).alloc_owned(value)
    }

    #[inline]
    unsafe fn build_thread_state_unchecked(&self) -> Self::ThreadState {
        (**self).build_thread_state_unchecked()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ReclaimThreadState (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub unsafe trait ReclaimThreadState<T> {
    type Reclaim: Reclaim<T>;
    type Guard: Protect<T, Reclaim = Self::Reclaim> + ProtectExt<T>;

    fn build_guard(&self) -> Self::Guard;
    fn alloc_owned<N: Unsigned>(&self, value: T) -> Owned<T, Self::Reclaim, N>;
    unsafe fn retire_record(&self, retired: Retired<Self::Reclaim>);
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Protect (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A trait for implementing guard types associated with a specific [`Reclaim`]
/// mechanism.
pub unsafe trait Protect<T>: Clone {
    /// The associated [`Reclaim`] mechanism.
    type Reclaim: Reclaim<T>;

    /// Loads and protects the value currently stored in `src` and returns a
    /// protected [`Shared`] pointer to it.
    ///
    /// `protect` takes an [`Ordering`] argument...
    fn protect<N: Unsigned>(
        &mut self,
        atomic: &Atomic<T, Self::Reclaim, N>,
        order: Ordering,
    ) -> Protected<T, Self::Reclaim, N>;

    /// Loads and protects the value currently stored in `src` if it equals the
    /// `expected` value and returns a protected [`Shared`] pointer to it.
    ///
    /// `protect_if_equal` takes an [`Ordering`] argument...
    fn protect_if_equal<N: Unsigned>(
        &mut self,
        atomic: &Atomic<T, Self::Reclaim, N>,
        expected: MarkedPtr<T, N>,
        order: Ordering,
    ) -> Result<Protected<T, Self::Reclaim, N>, NotEqual>;
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ProtectExt (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait ProtectExt<T>: Protect<T> {
    fn adopt<'g, N: Unsigned>(
        &'g mut self,
        fused: FusedGuardRef<'_, T, Self, N>,
    ) -> Shared<'g, T, Self::Reclaim, N>;

    fn protect_and_fuse_ref<N: Unsigned>(
        &mut self,
        atomic: &Atomic<T, Self::Reclaim, N>,
        order: Ordering,
    ) -> Maybe<FusedGuardRef<T, Self, N>>;

    fn protect_and_fuse_ref_if_equal<N: Unsigned>(
        &mut self,
        atomic: &Atomic<T, Self::Reclaim, N>,
        expected: MarkedPtr<T, N>,
        order: Ordering,
    ) -> Result<Maybe<FusedGuardRef<T, Self, N>>, NotEqual>;
}

/********** blanket impl **************************************************************************/

impl<T, G> ProtectExt<T> for G
where
    G: Protect<T>,
{
    #[inline]
    fn adopt<'g, N: Unsigned>(
        &'g mut self,
        fused: FusedGuardRef<'_, T, Self, N>,
    ) -> Shared<'g, T, Self::Reclaim, N> {
        mem::swap(self, fused.guard);
        unsafe { Shared::from_marked_non_null(fused.shared) }
    }

    #[inline]
    fn protect_and_fuse_ref<N: Unsigned>(
        &mut self,
        atomic: &Atomic<T, Self::Reclaim, N>,
        order: Ordering,
    ) -> Maybe<FusedGuardRef<T, Self, N>> {
        match self.protect(atomic, order).shared() {
            Maybe::Some(shared) => {
                let shared = shared.inner;
                Maybe::Some(FusedGuardRef { guard: self, shared })
            }
            Maybe::Null(tag) => Maybe::Null(tag),
        }
    }

    #[inline]
    fn protect_and_fuse_ref_if_equal<N: Unsigned>(
        &mut self,
        atomic: &Atomic<T, Self::Reclaim, N>,
        expected: MarkedPtr<T, N>,
        order: Ordering,
    ) -> Result<Maybe<FusedGuardRef<T, Self, N>>, NotEqual> {
        match self.protect_if_equal(atomic, expected, order) {
            Ok(protected) => match protected.shared() {
                Maybe::Some(shared) => {
                    let shared = shared.inner;
                    Ok(Maybe::Some(FusedGuardRef { guard: self, shared }))
                }
                Maybe::Null(tag) => Ok(Maybe::Null(tag)),
            },
            Err(e) => Err(e),
        }
    }
}

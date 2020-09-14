use core::mem;
use core::ops::Deref;
use core::sync::atomic::Ordering;

#[cfg(not(feature = "std"))]
use alloc::boxed::Box;

use conquer_pointer::{MarkedPtr, Null};

use crate::alias::RetiredRecord;
use crate::atomic::Atomic;
use crate::fused::{FusedProtected, FusedProtectedRef, FusedShared, FusedSharedRef};
use crate::{Maybe, NotEqual, Owned, Protected, Retired, Shared};

/********** macros ********************************************************************************/

#[macro_export]
macro_rules! impl_typed_reclaim {
    ($reclaim:ty, $header:ty) => {
        unsafe impl<T> $crate::ReclaimBase for $reclaim {
            type Header = $header;
            type Retired = T;
        }

        unsafe impl<T> $crate::Reclaim<T> for $reclaim {
            #[inline(always)]
            unsafe fn retire(ptr: *mut T) -> *mut T {
                ptr
            }
        }
    };
}

#[macro_export]
macro_rules! impl_dyn_reclaim {
    ($reclaim:ty, $header:ty) => {
        unsafe impl $crate::ReclaimBase for $reclaim {
            type Header = $header;
            type Retired = core::any::Any;
        }

        unsafe impl<T> $crate::Reclaim<T> for $reclaim {
            #[inline(always)]
            unsafe fn retire(ptr: *mut T) -> *mut dyn core::any::Any {
                ptr as *mut _
            }
        }
    };
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ReclaimBase (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub unsafe trait ReclaimBase: Sync + Sized {
    type Header: Sized + 'static;
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

    fn alloc_owned<const N: usize>(&self, value: T) -> Owned<T, Self::Reclaim, N>;
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
    fn alloc_owned<const N: usize>(&self, value: T) -> Owned<T, Self::Reclaim, N> {
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

    fn derived_from(&self, reclaimer: &impl ReclaimRef<T, Reclaim = Self::Reclaim>) -> bool;
    fn build_guard(&self) -> Self::Guard;
    fn alloc_owned<const N: usize>(&self, value: T) -> Owned<T, Self::Reclaim, N>;
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
    fn protect<const N: usize>(
        &mut self,
        atomic: &Atomic<T, Self::Reclaim, N>,
        order: Ordering,
    ) -> Protected<T, Self::Reclaim, N>;

    /// Loads and protects the value currently stored in `src` if it equals the
    /// `expected` value and returns a protected [`Shared`] pointer to it.
    ///
    /// `protect_if_equal` takes an [`Ordering`] argument...
    fn protect_if_equal<const N: usize>(
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
    fn adopt_ref<const N: usize>(
        &mut self,
        fused: FusedSharedRef<'_, T, Self, N>,
    ) -> Shared<T, Self::Reclaim, N>;

    fn protect_fused<const N: usize>(
        self,
        atomic: &Atomic<T, Self::Reclaim, N>,
        order: Ordering,
    ) -> FusedProtected<T, Self, N>;

    fn protect_fused_if_equal<const N: usize>(
        self,
        atomic: &Atomic<T, Self::Reclaim, N>,
        expected: MarkedPtr<T, N>,
        order: Ordering,
    ) -> Result<FusedProtected<T, Self, N>, (Self, NotEqual)>;

    fn protect_fused_ref<const N: usize>(
        &mut self,
        atomic: &Atomic<T, Self::Reclaim, N>,
        order: Ordering,
    ) -> Maybe<FusedSharedRef<T, Self, N>>;

    fn protect_fused_ref_if_equal<const N: usize>(
        &mut self,
        atomic: &Atomic<T, Self::Reclaim, N>,
        expected: MarkedPtr<T, N>,
        order: Ordering,
    ) -> Result<FusedProtectedRef<T, Self, N>, NotEqual>;

    /*fn adopt_fused_shared_ref<'g, const N: usize>(
        &'g mut self,
        fused: FusedSharedRef<'_, T, Self, N>,
    ) -> Shared<'g, T, Self::Reclaim, N>;

    fn adopt_ref<'g, const N: usize>(
        &'g mut self,
        fused: FusedGuardRef<'_, T, Self, N>,
    ) -> Shared<'g, T, Self::Reclaim, N>;

    fn adopt_fused(self, fused: FusedGuardRef<'_, T, Self, N>) -> FusedGuard<T, Self, N>;

    fn protect_fused_ref<const N: usize>(
        &mut self,
        atomic: &Atomic<T, Self::Reclaim, N>,
        order: Ordering,
    ) -> Maybe<FusedGuardRef<T, Self, N>>;

    fn protect_fused_ref_if_equal<const N: usize>(
        &mut self,
        atomic: &Atomic<T, Self::Reclaim, N>,
        expected: MarkedPtr<T, N>,
        order: Ordering,
    ) -> Result<Maybe<FusedGuardRef<T, Self, N>>, NotEqual>;*/
}

/********** blanket impl **************************************************************************/

impl<T, G> ProtectExt<T> for G
where
    G: Protect<T>,
{
    fn adopt_ref<const N: usize>(
        &mut self,
        fused: FusedSharedRef<'_, T, Self, N>,
    ) -> Shared<T, Self::Reclaim, N> {
        todo!()
    }

    fn protect_fused<const N: usize>(
        self,
        atomic: &Atomic<T, Self::Reclaim, N>,
        order: Ordering,
    ) -> FusedProtected<T, Self, N> {
        todo!()
    }

    fn protect_fused_if_equal<const N: usize>(
        self,
        atomic: &Atomic<T, Self::Reclaim, N>,
        expected: MarkedPtr<T, N>,
        order: Ordering,
    ) -> Result<FusedProtected<T, Self, N>, (Self, NotEqual)> {
        todo!()
    }

    fn protect_fused_ref<const N: usize>(
        &mut self,
        atomic: &Atomic<T, Self::Reclaim, N>,
        order: Ordering,
    ) -> Maybe<FusedSharedRef<T, Self, N>> {
        todo!()
    }

    fn protect_fused_ref_if_equal<const N: usize>(
        &mut self,
        atomic: &Atomic<T, Self::Reclaim, N>,
        expected: MarkedPtr<T, N>,
        order: Ordering,
    ) -> Result<FusedProtectedRef<T, Self, N>, NotEqual> {
        todo!()
    }
}

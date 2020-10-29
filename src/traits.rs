use core::ops::Deref;
use core::sync::atomic::Ordering;

#[cfg(not(feature = "std"))]
use alloc::boxed::Box;

use conquer_pointer::MarkedPtr;

use crate::alias::RetiredRecord;
use crate::atomic::Atomic;
use crate::fused::{FusedProtected, FusedProtectedRef};
use crate::{NotEqual, Owned, Protected, Retired};

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

/// A trait implementing the basic functionality of a memory reclamation scheme.
pub unsafe trait ReclaimBase: Sync + Sized {
    /// The header type that is allocated alongside every
    /// [`Owned`][crate::Owned].
    type Header: Sized + 'static;
    /// The type that can be retired and reclaimed by this reclamation scheme
    /// implementation.
    type Retired: ?Sized;

    /// Reclaims the `retired` record.
    ///
    /// # Safety
    ///
    /// `retired` must point at a record that was allocated through the same
    /// memory reclamation type, was later retired and has not been reclaimed
    /// previously.
    /// The function must only be called, if the reclamation scheme can prove
    /// that there are protected ([`Shared`](crate::Shared)) references anymore
    /// in any thread.
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

/// An extension for the [`ReclaimBase`] trait, which implements the
/// functionality of *retiring* memory records of the type (`T`) the trait is
/// implemented for.
///
/// In general, a *typed* reclamation mechanism (i.e., a mechanism specific for
/// only one type) will only implement this trait for its own type, whereas an
/// *untyped* reclamation mechanism will implement this trait for any type.
pub unsafe trait Reclaim<T>: ReclaimBase {
    /// Retires the memory record pointed to by `ptr`.
    ///
    /// In general, a pointer given to this function should always be derived
    /// from an [`Unlinked`](crate::Unlinked) through the
    /// [`into_retired`](crate::Unlinked::into_retired) method.
    ///
    /// # Safety
    ///
    /// The given `ptr` must point at a live and unlinked memory record that had
    /// been allocated as a record for the same [`ReclaimBase`].
    unsafe fn retire(ptr: *mut T) -> *mut Self::Retired;
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ReclaimRef (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A trait for instances of the global state of a reclamation mechanism or,
/// alternatively, a reference or handle to such an instance.
pub unsafe trait ReclaimRef<T>: Sync + Sized {
    /// The associated reclamation mechanism.
    type Reclaim: Reclaim<T>;
    /// The associated per-thread state.
    type ThreadState: ReclaimThreadState<T, Reclaim = Self::Reclaim>;

    /// Allocates an owned record with an appropriate [`Header`][ReclaimBase::Header].
    fn alloc_owned<const N: usize>(&self, value: T) -> Owned<T, Self::Reclaim, N>;
    /// Builds a an instance of the associated per-thread state, which *may*
    /// contain a non lifetime-checked reference (e.g. a raw pointer) to `self`.
    ///
    /// The rationale for this method being `unsafe` is two-fold:
    ///   1. The per-thread state will often be stored along-side an owning
    ///      pointer (e.g., an [`Arc`][alloc::sync::Arc]) to the global state,
    ///      within the same `struct`, which would require some sort of
    ///      self-referential type, which is currently not possible in Rust.
    ///   2. Returning a per-thread state instance with a lifetime-checked
    ///      reference to `self` from a trait would require generic associated
    ///      types (GAT), which are currently not available in *stable* Rust.
    ///
    /// # Safety
    ///
    /// The returned thread state instance **must not** outlive `self`.
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

/// A trait implementing the functionality of the per-thread state of a
/// reclamation mechanism.
pub unsafe trait ReclaimThreadState<T> {
    /// The associated reclamation mechanism.
    type Reclaim: Reclaim<T>;
    /// The associated guard type.
    type Guard: Protect<T, Reclaim = Self::Reclaim> + ProtectExt<T>;

    /// Returns `true` if `self` has been created from the given `reclaimer`
    /// (global state instance) reference.
    fn derived_from(&self, reclaimer: &impl ReclaimRef<T, Reclaim = Self::Reclaim>) -> bool;
    /// Builds a guard instance.
    fn build_guard(&self) -> Self::Guard;
    /// Allocates an owned record with an appropriate
    /// [`Header`][ReclaimBase::Header].
    fn alloc_owned<const N: usize>(&self, value: T) -> Owned<T, Self::Reclaim, N>;
    /// Retires an [`Unlinked`][crate::Unlinked] memory record.
    unsafe fn retire_record(&self, retired: Retired<Self::Reclaim>);
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Protect (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A trait for implementing guard types associated with a specific [`Reclaim`]
/// mechanism.
pub unsafe trait Protect<T>: Clone {
    /// The associated reclamation mechanism.
    type Reclaim: Reclaim<T>;

    /// Loads and protects the value currently stored in `src` and returns a
    /// protected [`Shared`](crate::Shared) pointer to it.
    ///
    /// `protect` takes an [`Ordering`] argument...
    fn protect<const N: usize>(
        &mut self,
        atomic: &Atomic<T, Self::Reclaim, N>,
        order: Ordering,
    ) -> Protected<T, Self::Reclaim, N>;

    /// Loads and protects the value currently stored in `src` if it equals the
    /// `expected` value and returns a protected [`Shared`](crate::Shared)
    /// pointer to it.
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
    ) -> FusedProtectedRef<T, Self, N>;

    fn protect_fused_ref_if_equal<const N: usize>(
        &mut self,
        atomic: &Atomic<T, Self::Reclaim, N>,
        expected: MarkedPtr<T, N>,
        order: Ordering,
    ) -> Result<FusedProtectedRef<T, Self, N>, NotEqual>;
}

/********** blanket impl **************************************************************************/

impl<T, G> ProtectExt<T> for G
where
    G: Protect<T>,
{
    #[inline]
    fn protect_fused<const N: usize>(
        mut self,
        atomic: &Atomic<T, Self::Reclaim, N>,
        order: Ordering,
    ) -> FusedProtected<T, Self, N> {
        let protected = self.protect(atomic, order).into_marked_ptr();
        FusedProtected { guard: self, protected }
    }

    #[inline]
    fn protect_fused_if_equal<const N: usize>(
        mut self,
        atomic: &Atomic<T, Self::Reclaim, N>,
        expected: MarkedPtr<T, N>,
        order: Ordering,
    ) -> Result<FusedProtected<T, Self, N>, (Self, NotEqual)> {
        match self.protect(atomic, order).into_marked_ptr() {
            protected if protected == expected => Ok(FusedProtected { guard: self, protected }),
            _ => Err((self, NotEqual)),
        }
    }

    #[inline]
    fn protect_fused_ref<const N: usize>(
        &mut self,
        atomic: &Atomic<T, Self::Reclaim, N>,
        order: Ordering,
    ) -> FusedProtectedRef<T, Self, N> {
        let protected = self.protect(atomic, order).into_marked_ptr();
        FusedProtectedRef { guard: self, protected }
    }

    #[inline]
    fn protect_fused_ref_if_equal<const N: usize>(
        &mut self,
        atomic: &Atomic<T, Self::Reclaim, N>,
        expected: MarkedPtr<T, N>,
        order: Ordering,
    ) -> Result<FusedProtectedRef<T, Self, N>, NotEqual> {
        match self.protect(atomic, order).into_marked_ptr() {
            protected if protected == expected => Ok(FusedProtectedRef { guard: self, protected }),
            _ => Err(NotEqual),
        }
    }
}

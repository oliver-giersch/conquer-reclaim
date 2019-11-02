//! Internal macros and traits which may appear in public interfaces, but are
//! not actually exported by the crate.

use conquer_pointer::{MarkedNonNullable, MarkedOption};
use typenum::Unsigned;

use crate::traits::{Reclaim, SharedPointer};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Internal (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A general purpose sealed marker trait for all relevant types of this crate.
pub trait Internal {}

/********** blanket impls *************************************************************************/

impl<P, T, R: Reclaim, N: Unsigned> Internal for Option<P> where
    P: SharedPointer<Item = T, Reclaimer = R, MarkBits = N>
        + MarkedNonNullable<Item = T, MarkBits = N>
{
}

impl<P, T, R: Reclaim, N: Unsigned> Internal for MarkedOption<P> where
    P: SharedPointer<Item = T, Reclaimer = R, MarkBits = N>
        + MarkedNonNullable<Item = T, MarkBits = N>
{
}

/********** implementation macros *****************************************************************/

macro_rules! impl_common {
    () => {
        /// Creates a `None` variant for this type.
        ///
        /// # Examples
        ///
        /// When storing a `null` pointer into an [`Atomic`][atomic] by calling
        /// e.g. [`store`][store] or one of the swapping methods, the compiler
        /// is usually unable to infer the correct type for a simple
        /// [`None`][Option::None] argument.
        ///
        /// ```
        /// use std::sync::atomic::Ordering;
        ///
        /// type Atomic<T> = reclaim::leak::Atomic<T, reclaim::typenum::U0>;
        /// type Owned<T> = reclaim::leak::Owned<T, reclaim::typenum::U0>;
        /// type Unlinked<T> = reclaim::leak::Unlinked<T, reclaim::typenum::U0>;
        ///
        /// let atomic = Atomic::new(1);
        /// let swap = atomic.swap(Owned::none(), Ordering::Relaxed).unwrap();
        ///
        /// assert_eq!(swap.as_ref(), &1);
        /// unsafe { Unlinked::retire(swap) }; // leaks memory
        /// ```
        ///
        /// [atomic]: crate::atomic::Atomic
        /// [store]:  crate::atomic::Atomic::store
        #[inline]
        pub fn none() -> Option<Self> {
            None
        }
    };
}

macro_rules! impl_marked_non_nullable {
    () => {
        type MarkBits = N;

        #[inline]
        fn into_marked_non_null(self) -> MarkedNonNull<Self::Item, Self::MarkBits> {
            let inner = self.inner;
            #[allow(clippy::forget_copy)]
            core::mem::forget(self);
            inner
        }

        #[inline]
        fn decompose(&self) -> (NonNull<Self::Item>, usize) {
            self.inner.decompose()
        }

        #[inline]
        fn decompose_ptr(&self) -> *mut Self::Item {
            self.inner.decompose_ptr()
        }

        #[inline]
        fn decompose_non_null(&self) -> NonNull<Self::Item> {
            self.inner.decompose_non_null()
        }

        #[inline]
        fn decompose_tag(&self) -> usize {
            self.inner.decompose_tag()
        }
    };
}

macro_rules! impl_non_nullable {
    () => {
        type Item = T;

        #[inline]
        fn as_const_ptr(&self) -> *const Self::Item {
            self.inner.decompose_ptr() as *const _
        }

        #[inline]
        fn as_mut_ptr(&self) -> *mut Self::Item {
            self.inner.decompose_ptr()
        }

        #[inline]
        fn as_non_null(&self) -> NonNull<Self::Item> {
            self.inner.decompose_non_null()
        }
    };
}

macro_rules! impl_shared_pointer {
    () => {
        type Item = T;
        type Reclaimer = R;
        type MarkBits = N;
        type Pointer = Self;

        #[inline]
        fn with(ptr: Self::Pointer) -> Self {
            ptr
        }

        #[inline]
        fn compose(ptr: Self::Pointer, tag: usize) -> Self {
            let inner = ptr.inner.set_tag(tag);
            #[allow(clippy::forget_copy)]
            core::mem::forget(ptr);
            Self { inner, _marker: core::marker::PhantomData }
        }

        #[inline]
        unsafe fn from_marked_ptr(marked_ptr: MarkedPtr<Self::Item, Self::MarkBits>) -> Self {
            debug_assert!(!marked_ptr.is_null());
            Self {
                inner: conquer_pointer::MarkedNonNull::new_unchecked(marked_ptr),
                _marker: core::marker::PhantomData,
            }
        }

        #[inline]
        unsafe fn from_marked_non_null(
            marked_ptr: MarkedNonNull<Self::Item, Self::MarkBits>
        ) -> Self {
            Self { inner: marked_ptr, _marker: core::marker::PhantomData }
        }

        #[inline]
        fn as_marked_ptr(&self) -> MarkedPtr<Self::Item, Self::MarkBits> {
            self.inner.into_marked_ptr()
        }

        #[inline]
        fn into_marked_ptr(self) -> MarkedPtr<Self::Item, Self::MarkBits> {
            let inner = self.inner;
            #[allow(clippy::forget_copy)]
            core::mem::forget(self);
            inner.into_marked_ptr()
        }

        #[inline]
        fn clear_tag(self) -> Self {
            let inner = self.inner.clear_tag();
            #[allow(clippy::forget_copy)]
            core::mem::forget(self);
            Self { inner, _marker: core::marker::PhantomData }
        }

        #[inline]
        fn set_tag(self, tag: usize) -> Self {
            let inner = self.inner.set_tag(tag);
            #[allow(clippy::forget_copy)]
            core::mem::forget(self);
            Self { inner, _marker: core::marker::PhantomData }
        }

        #[inline]
        fn decompose(self) -> (Self, usize) {
            let inner = self.inner;
            let tag = inner.decompose_tag();
            #[allow(clippy::forget_copy)]
            core::mem::forget(self);
            (Self { inner: inner.clear_tag(), _marker: core::marker::PhantomData}, tag)
        }
    };
}

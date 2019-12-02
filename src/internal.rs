//! Internal macros and traits which may appear in public interfaces, but are
//! not actually exported by the crate.

use conquer_pointer::{MarkedNonNullable, MarkedOption};
use typenum::Unsigned;

use crate::traits::{Reclaimer, SharedPointer};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Internal (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A general purpose sealed marker trait for all relevant types of this crate.
pub trait Internal {}

/********** blanket impls *************************************************************************/

impl<P, T, R: Reclaimer, N: Unsigned> Internal for Option<P> where
    P: SharedPointer<Item = T, Reclaimer = R, MarkBits = N>
        + MarkedNonNullable<Item = T, MarkBits = N>
{
}

impl<P, T, R: Reclaimer, N: Unsigned> Internal for MarkedOption<P> where
    P: SharedPointer<Item = T, Reclaimer = R, MarkBits = N>
        + MarkedNonNullable<Item = T, MarkBits = N>
{
}

/********** implementation macros *****************************************************************/

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

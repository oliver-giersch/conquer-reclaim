//! Internal macros and traits which may appear in public interfaces, but are
//! not actually exported by the crate.

////////////////////////////////////////////////////////////////////////////////////////////////////
// Internal (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A general purpose sealed marker trait for all relevant types of this crate.
pub trait Internal {}

macro_rules! impl_reclaim_pointer {
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
        fn compose(ptr: Self, tag: usize) -> Self {
            let inner = ptr.inner.with_tag(tag);
            core::mem::forget(ptr);
            Self { inner, _marker: core::marker::PhantomData }
        }

        #[inline]
        unsafe fn from_marked_ptr(
            marked_ptr: conquer_pointer::MarkedPtr<Self::Item, Self::MarkBits>
        ) -> Self {
            debug_assert!(!marked_ptr.is_null());
            Self {
                inner: conquer_pointer::MarkedNonNull::new_unchecked(marked_ptr),
                _marker: core::marker::PhantomData,
            }
        }

        #[inline]
        unsafe fn from_marked_non_null(
            marked_ptr: conquer_pointer::MarkedNonNull<Self::Item, Self::MarkBits>
        ) -> Self {
            Self { inner: marked_ptr, _marker: core::marker::PhantomData }
        }

        #[inline]
        fn as_marked_ptr(&self) -> conquer_pointer::MarkedPtr<Self::Item, Self::MarkBits> {
            self.inner.into_marked_ptr()
        }

        #[inline]
        fn into_marked_ptr(self) -> conquer_pointer::MarkedPtr<Self::Item, Self::MarkBits> {
            let inner = self.inner;
            core::mem::forget(self);
            inner.into_marked_ptr();
        }

        #[inline]
        fn into_unmarked(self) -> Self {
            let inner = self.inner.clear_tag();
            core::mem::forget(self);
            Self { inner, _marker: core::marker::PhantomData }
        }

        #[inline]
        fn decompose(self) -> (Self, usize) {
            let (inner, tag) = self.inner.decompose();
            core::mem::forget(self);
            (Self { inner, _marker: core::marker::PhantomData}, tag)
        }
    };
}

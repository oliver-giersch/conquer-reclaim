macro_rules! impl_marked_non_nullable {
    () => {
        type MarkBits = N;

        #[inline]
        fn as_marked_ptr(ptr: &Self) -> conquer_pointer::MarkedPtr<Self::Item, Self::MarkBits> {
            ptr.into_marked_ptr()
        }

        #[inline]
        fn into_marked_ptr(ptr: Self) -> conquer_pointer::MarkedPtr<Self::Item, Self::MarkBits> {
            ptr.into_marked_ptr()
        }

        #[inline]
        fn clear_tag(ptr: Self) -> Self {
            ptr.clear_tag()
        }

        #[inline]
        fn split_tag(ptr: Self) -> (Self, usize) {
            ptr.split_tag()
        }

        #[inline]
        fn set_tag(ptr: Self, tag: usize) -> Self {
            ptr.set_tag(tag)
        }

        #[inline]
        fn decompose(ptr: &Self) -> (core::ptr::NonNull<Self::Item>, usize) {
            ptr.inner.decompose()
        }

        #[inline]
        fn decompose_ptr(ptr: &Self) -> *mut Self::Item {
            ptr.inner.decompose_ptr()
        }

        #[inline]
        fn decompose_non_null(ptr: &Self) -> core::ptr::NonNull<Self::Item> {
            ptr.inner.decompose_non_null()
        }

        #[inline]
        fn decompose_tag(ptr: &Self) -> usize {
            ptr.inner.decompose_tag()
        }
    };
}

macro_rules! impl_non_nullable {
    () => {
        type Item = T;

        #[inline]
        fn as_const_ptr(ptr: &Self) -> *const Self::Item {
            ptr.inner.decompose_ptr() as *const _
        }

        #[inline]
        fn as_mut_ptr(ptr: &Self) -> *mut Self::Item {
            ptr.inner.decompose_ptr()
        }

        #[inline]
        fn as_non_null(ptr: &Self) -> core::ptr::NonNull<Self::Item> {
            ptr.inner.decompose_non_null()
        }
    };
}

macro_rules! impl_common_from {
    () => {
        #[inline]
        pub unsafe fn from_marked_ptr(ptr: MarkedPtr<T, N>) -> Self {
            Self { inner: MarkedNonNull::new_unchecked(ptr), _marker: PhantomData }
        }

        #[inline]
        pub unsafe fn from_marked_non_null(ptr: MarkedNonNull<T, N>) -> Self {
            Self { inner: ptr, _marker: PhantomData }
        }
    };
}

macro_rules! impl_common {
    () => {
        #[inline]
        pub fn into_marked_ptr(self) -> MarkedPtr<T, N> {
            self.inner.into_marked_ptr()
        }

        #[inline]
        pub fn clear_tag(self) -> Self {
            Self { inner: self.inner.clear_tag(), _marker: PhantomData }
        }

        #[inline]
        pub fn split_tag(self) -> (Self, usize) {
        let (inner, tag) = self.inner.split_tag();
            (Self { inner, _marker: PhantomData }, tag)
        }

        #[inline]
        pub fn set_tag(self, tag: usize) -> Self {
            Self { inner: self.inner.set_tag(tag), _marker: PhantomData }
        }

        #[inline]
        pub fn update_tag(self, func: impl FnOnce(usize) -> usize) -> Self {
            unimplemented!()
        }

        #[inline]
        pub fn decompose_tag(self) -> usize {
            self.inner.decompose_tag()
        }
    };
}

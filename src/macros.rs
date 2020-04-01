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
            self.inner.into()
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
            Self { inner: self.inner.update_tag(func), _marker: PhantomData }
        }

        #[inline]
        pub fn decompose_tag(self) -> usize {
            self.inner.decompose_tag()
        }
    };
}

macro_rules! impl_fmt_debug {
    ($ty_name:ident) => {
        #[inline]
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            let (ptr, tag) = self.inner.decompose();
            f.debug_struct(stringify!($ty_name)).field("ptr", &ptr).field("tag", &tag).finish()
        }
    };
}

macro_rules! impl_fmt_pointer {
    () => {
        #[inline]
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            fmt::Pointer::fmt(&self.inner, f)
        }
    };
}

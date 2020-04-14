use core::fmt;
use core::marker::PhantomData;
use core::mem::ManuallyDrop;
use core::ptr;

use conquer_pointer::typenum::Unsigned;
use conquer_pointer::{MarkedNonNull, MarkedPtr};

use crate::retired::Retired;
use crate::traits::{GlobalReclaim, Reclaim};

use crate::Unlinked;

/********** impl inherent *************************************************************************/

impl<T, R: Reclaim, N: Unsigned> Unlinked<T, R, N> {
    impl_from_ptr!();
    impl_from_non_null!();

    #[inline]
    pub fn as_marked_ptr(&self) -> MarkedPtr<T, N> {
        self.inner.into()
    }

    impl_common!();

    #[inline]
    pub fn into_retired(self) -> Retired<R>
    where
        T: 'static,
    {
        unsafe { self.into_retired_unchecked() }
    }

    #[inline]
    pub unsafe fn into_retired_unchecked(self) -> Retired<R> {
        Retired::new_unchecked(self.inner.decompose_non_null())
    }

    #[inline]
    pub unsafe fn take<U>(&self, take: impl (FnOnce(&T) -> &ManuallyDrop<U>) + 'static) -> U {
        let src = take(self.as_ref());
        ptr::read(&**src)
    }

    #[inline]
    pub unsafe fn as_ref(&self) -> &T {
        self.inner.as_ref()
    }

    #[inline]
    pub unsafe fn decompose_ref(&self) -> (&T, usize) {
        self.inner.decompose_ref()
    }

    #[inline]
    pub unsafe fn cast<U>(self) -> Unlinked<U, R, N> {
        Unlinked { inner: self.inner.cast(), _marker: PhantomData }
    }
}

impl<T, R: GlobalReclaim, N: Unsigned> Unlinked<T, R, N> {
    #[inline]
    pub unsafe fn retire(self)
    where
        T: 'static,
    {
        self.retire_unchecked()
    }

    #[inline]
    pub unsafe fn retire_unchecked(self) {
        R::retire_record(self.into_retired_unchecked())
    }
}

/********** impl Debug ****************************************************************************/

impl<T, R, N: Unsigned> fmt::Debug for Unlinked<T, R, N> {
    impl_fmt_debug!(Unlinked);
}

/********** impl Pointer **************************************************************************/

impl<T, R, N: Unsigned> fmt::Pointer for Unlinked<T, R, N> {
    impl_fmt_pointer!();
}

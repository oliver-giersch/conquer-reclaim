use core::fmt;
use core::marker::PhantomData;
use core::mem::ManuallyDrop;
use core::ptr;

use conquer_pointer::{MarkedNonNull, MarkedPtr};

use crate::retired::Retired;
use crate::traits::Reclaim;

use crate::Unlinked;

/********** impl inherent *************************************************************************/

impl<T, R, const N: usize> Unlinked<T, R, N> {
    impl_from_ptr!();
    impl_from_non_null!();

    #[inline]
    pub fn as_marked_ptr(&self) -> MarkedPtr<T, N> {
        self.inner.into()
    }

    impl_common!();

    #[inline]
    pub unsafe fn take<U>(&self, take: impl (FnOnce(&T) -> &ManuallyDrop<U>)) -> U {
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

impl<T, R: Reclaim<T>, const N: usize> Unlinked<T, R, N> {
    #[inline]
    pub fn into_retired(self) -> Retired<R> {
        let ptr = self.inner.decompose_ptr();
        unsafe {
            let retired = <R as Reclaim<T>>::retire(ptr);
            Retired::new_unchecked(retired)
        }
    }
}

/********** impl Debug ****************************************************************************/

impl<T, R, const N: usize> fmt::Debug for Unlinked<T, R, N> {
    impl_fmt_debug!(Unlinked);
}

/********** impl Pointer **************************************************************************/

impl<T, R, const N: usize> fmt::Pointer for Unlinked<T, R, N> {
    impl_fmt_pointer!();
}

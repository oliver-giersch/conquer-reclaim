use core::marker::PhantomData;
use core::ptr::NonNull;

use conquer_pointer::NonNullable;
use typenum::Unsigned;

use crate::Unprotected;

/********** impl Clone ****************************************************************************/

impl<T, R, N> Clone for Unprotected<T, R, N> {
    #[inline]
    fn clone(&self) -> Self {
        Self { inner: self.inner, _marker: PhantomData }
    }
}

/********** impl Copy *****************************************************************************/

impl<T, R, N> Copy for Unprotected<T, R, N> {}

/********** impl NonNullable **********************************************************************/

impl<T, R, N: Unsigned> NonNullable for Unprotected<T, R, N> {
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
}

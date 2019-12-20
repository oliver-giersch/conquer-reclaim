use core::fmt;
use core::marker::PhantomData;

use conquer_pointer::MarkedPtr;

use crate::traits::Reclaim;
use crate::typenum::Unsigned;
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

/********** impl inherent *************************************************************************/

impl<T, R, N> Unprotected<T, R, N> {
    #[inline]
    pub const fn null() -> Self {
        Self { inner: MarkedPtr::null(), _marker: PhantomData }
    }
}

impl<T, R: Reclaim, N: Unsigned + 'static> Unprotected<T, R, N> {
    #[inline]
    pub fn is_null(self) -> bool {
        self.inner.is_null()
    }

    impl_common!();

    #[inline]
    pub fn cast<U>(self) -> Unprotected<U, R, N> {
        Unprotected { inner: self.inner.cast(), _marker: PhantomData }
    }
}

/********** impl Debug ****************************************************************************/

impl<T, R: Reclaim, N: Unsigned + 'static> fmt::Debug for Unprotected<T, R, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (ptr, tag) = self.inner.decompose();
        f.debug_struct("Unprotected").field("ptr", &ptr).field("tag", &tag).finish()
    }
}

/********** impl Pointer **************************************************************************/

impl<T, R: Reclaim, N: Unsigned + 'static> fmt::Pointer for Unprotected<T, R, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.inner.decompose_ptr(), f)
    }
}

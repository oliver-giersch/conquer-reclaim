use core::fmt;
use core::marker::PhantomData;

use conquer_pointer::MarkedPtr;
use typenum::Unsigned;

use crate::traits::Reclaimer;
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

impl<T, R: Reclaimer, N: Unsigned> Unprotected<T, R, N> {
    #[inline]
    pub fn is_null(self) -> bool {
        self.inner.is_null()
    }
}

/********** impl Debug ****************************************************************************/

impl<T, R: Reclaimer, N: Unsigned> fmt::Debug for Unprotected<T, R, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (ptr, tag) = self.inner.decompose();
        f.debug_struct("Unprotected").field("ptr", &ptr).field("tag", &tag).finish()
    }
}

/********** impl Pointer **************************************************************************/

impl<T, R: Reclaimer, N: Unsigned> fmt::Pointer for Unprotected<T, R, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.inner.decompose_ptr(), f)
    }
}

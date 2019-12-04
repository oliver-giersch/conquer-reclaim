use core::fmt;
use core::marker::PhantomData;
use core::mem;
use core::ptr::NonNull;

use conquer_pointer::{MarkedNonNull, MarkedNonNullable, MarkedPtr, NonNullable};
use typenum::Unsigned;

use crate::internal::Internal;
use crate::traits::{Reclaimer, SharedPointer};
use crate::{Shared, Unprotected};

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

/********** impl MarkedNonNullable ****************************************************************/

/*impl<T, R, N: Unsigned> MarkedNonNullable for Unprotected<T, R, N> {
    impl_marked_non_nullable!();
}*/

/********** impl NonNullable **********************************************************************/

/*impl<T, R, N: Unsigned> NonNullable for Unprotected<T, R, N> {
    impl_non_nullable!();
}*/

/********** impl Internal *************************************************************************/

impl<T, R, N: Unsigned> Internal for Unprotected<T, R, N> {}

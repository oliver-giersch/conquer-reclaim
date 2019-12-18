use core::fmt;
use core::marker::PhantomData;

use conquer_pointer::{MarkedNonNull, MarkedNonNullable, MarkedPtr, NonNullable};

use crate::traits::Reclaimer;
use crate::typenum::Unsigned;
use crate::Shared;

/********** impl Clone ****************************************************************************/

impl<T, R, N> Clone for Shared<'_, T, R, N> {
    #[inline]
    fn clone(&self) -> Self {
        Self { inner: self.inner, _marker: PhantomData }
    }
}

/********** impl Copy *****************************************************************************/

impl<T, R, N> Copy for Shared<'_, T, R, N> {}

/********** impl inherent *************************************************************************/

impl<'g, T, R: Reclaimer, N: Unsigned + 'static> Shared<'g, T, R, N> {
    impl_common_from!();
    impl_common!();

    #[inline]
    pub unsafe fn deref(self) -> &'g T {
        self.inner.as_ref_unbounded()
    }

    #[inline]
    pub unsafe fn decompose_ref(self) -> (&'g T, usize) {
        self.inner.decompose_ref_unbounded()
    }

    #[inline]
    pub unsafe fn cast<'a, U>(self) -> Shared<'a, U, R, N> {
        Shared { inner: self.inner.cast(), _marker: PhantomData }
    }
}

/********** impl Debug ****************************************************************************/

impl<T: fmt::Debug, R, N: Unsigned + 'static> fmt::Debug for Shared<'_, T, R, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (ptr, tag) = self.inner.decompose();
        f.debug_struct("Shared").field("ptr", &ptr).field("tag", &tag).finish()
    }
}

/********** impl Pointer **************************************************************************/

impl<T, R: Reclaimer, N: Unsigned + 'static> fmt::Pointer for Shared<'_, T, R, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.inner.decompose_ptr(), f)
    }
}

/********** impl MarkedNonNullable ****************************************************************/

impl<T, R: Reclaimer, N: Unsigned + 'static> MarkedNonNullable for Shared<'_, T, R, N> {
    impl_marked_non_nullable!();
}

/********** impl NonNullable **********************************************************************/

impl<T, R, N: Unsigned + 'static> NonNullable for Shared<'_, T, R, N> {
    impl_non_nullable!();
}

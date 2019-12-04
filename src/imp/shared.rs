use core::fmt;
use core::marker::PhantomData;
use core::ptr::NonNull;

use conquer_pointer::{MarkedNonNullable, MarkedPtr, NonNullable};
use typenum::Unsigned;

use crate::internal::Internal;
use crate::traits::Reclaimer;
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

impl<'g, T, R: Reclaimer, N: Unsigned> Shared<'g, T, R, N> {
    impl_common_from!();
    impl_common!();

    #[inline]
    pub unsafe fn decompose_ref(self) -> (&'g T, usize) {
        self.inner.decompose_ref()
    }

    #[inline]
    pub unsafe fn deref(self) -> &'g T {
        self.inner.as_ref()
    }

    #[inline]
    pub unsafe fn cast<'a, U>(self) -> Shared<'a, U, R, N> {
        // TODO: check alignment of U
        unimplemented!()
    }
}

/********** impl Debug ****************************************************************************/

impl<T: fmt::Debug, R, N: Unsigned> fmt::Debug for Shared<'_, T, R, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Shared")
            .field("ref", self.as_ref())
            .field("tag", &self.inner.decompose_tag())
            .finish()
    }
}

/********** impl Pointer **************************************************************************/

impl<T, R: Reclaimer, N: Unsigned> fmt::Pointer for Shared<'_, T, R, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.inner.decompose_ptr(), f)
    }
}

/********** impl MarkedNonNullable ****************************************************************/

impl<T, R, N: Unsigned> MarkedNonNullable for Shared<'_, T, R, N> {
    impl_marked_non_nullable!();
}

/********** impl NonNullable **********************************************************************/

impl<T, R, N: Unsigned> NonNullable for Shared<'_, T, R, N> {
    impl_non_nullable!();
}

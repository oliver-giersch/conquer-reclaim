use core::ptr::NonNull;

use conquer_pointer::{MarkedNonNull, MarkedNonNullable, MarkedPtr, NonNullable};
use typenum::Unsigned;

use crate::internal::Internal;
use crate::retired::Retired;
use crate::traits::{Reclaimer, ReclaimerHandle, SharedPointer};
use crate::Unlinked;

/********** impl inherent *************************************************************************/

impl<T, R: Reclaimer, N: Unsigned> Unlinked<T, R, N> {
    /// TODO: Docs...
    #[inline]
    pub unsafe fn retire(self, handle: impl ReclaimerHandle<Reclaimer = R>)
    where
        T: 'static,
    {
        self.retire_unchecked(handle)
    }

    /// TODO: Docs...
    #[inline]
    pub unsafe fn retire_unchecked(self, handle: impl ReclaimerHandle<Reclaimer = R>) {
        let retired: Retired<R> = Retired::new_unchecked(self.inner.decompose_non_null());
        handle.retire(retired);
    }
}

/********** impl SharedPointer *******************************************************************/

impl<T, R: Reclaimer, N: Unsigned> SharedPointer for Unlinked<T, R, N> {
    impl_shared_pointer!();
}

/********** impl MarkedNonNullable ****************************************************************/

impl<T, R, N: Unsigned> MarkedNonNullable for Unlinked<T, R, N> {
    impl_marked_non_nullable!();
}

/********** impl NonNullable **********************************************************************/

impl<T, R, N: Unsigned> NonNullable for Unlinked<T, R, N> {
    impl_non_nullable!();
}

/********** impl Internal *************************************************************************/

impl<T, R, N: Unsigned> Internal for Unlinked<T, R, N> {}

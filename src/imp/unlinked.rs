use core::marker::PhantomData;

use conquer_pointer::{MarkedNonNull, MarkedNonNullable, MarkedPtr, NonNullable};
use typenum::Unsigned;

use crate::retired::Retired;
use crate::traits::{Reclaimer, ReclaimerHandle};
use crate::Unlinked;

/********** impl inherent *************************************************************************/

impl<T, R: Reclaimer, N: Unsigned> Unlinked<T, R, N> {
    impl_common_from!();
    impl_common!();

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

/********** impl MarkedNonNullable ****************************************************************/

impl<T, R: Reclaimer, N: Unsigned> MarkedNonNullable for Unlinked<T, R, N> {
    impl_marked_non_nullable!();
}

/********** impl NonNullable **********************************************************************/

impl<T, R, N: Unsigned> NonNullable for Unlinked<T, R, N> {
    impl_non_nullable!();
}

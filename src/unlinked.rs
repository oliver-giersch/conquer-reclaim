use typenum::Unsigned;

use crate::retired::Retired;
use crate::traits::{Reclaim, ReclaimHandle};
use crate::Unlinked;

/********** impl inherent *************************************************************************/

impl<T, R: Reclaim, N: Unsigned> Unlinked<T, R, N> {
    /// TODO: Docs...
    #[inline]
    pub unsafe fn retire(self, handle: &impl ReclaimHandle<Reclaimer = R>)
    where
        T: 'static,
    {
        self.retire_unchecked(handle)
    }

    /// TODO: Docs...
    #[inline]
    pub unsafe fn retire_unchecked(self, handle: &impl ReclaimHandle<Reclaimer = R>) {
        let retired: Retired<R> = Retired::new_unchecked(self.inner.decompose_non_null());
        handle.retire(retired);
    }
}

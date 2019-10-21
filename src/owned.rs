use core::convert::TryFrom;
use core::mem;
use core::ptr::NonNull;

use conquer_pointer::{MarkedNonNull, MarkedNonNullable, MarkedPtr, NonNullable};
use typenum::Unsigned;

use crate::atomic::Atomic;
use crate::internal::Internal;
use crate::record::Record;
use crate::traits::{Reclaim, SharedPointer};
use crate::Owned;

/********** impl Drop *****************************************************************************/

impl<T, R: Reclaim, N: Unsigned> Drop for Owned<T, R, N> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            let record = Record::<_, R>::from_raw(self.inner.decompose_ptr());
            mem::drop(Box::from_raw(record.as_ptr()));
        }
    }
}

/********** impl From *****************************************************************************/

impl<T, R: Reclaim, N: Unsigned> From<T> for Owned<T, R, N> {
    #[inline]
    fn from(owned: T) -> Self {
        unimplemented!()
    }
}

/********** impl SharedPointer *******************************************************************/

impl<T, R: Reclaim, N: Unsigned> SharedPointer for Owned<T, R, N> {
    impl_shared_pointer!();
}

/********** impl MarkedNonNullable ****************************************************************/

impl<T, R: Reclaim, N: Unsigned> MarkedNonNullable for Owned<T, R, N> {
    impl_marked_non_nullable!();
}

/********** impl NonNullable **********************************************************************/

impl<T, R: Reclaim, N: Unsigned> NonNullable for Owned<T, R, N> {
    impl_non_nullable!();
}

/********** impl Internal *************************************************************************/

impl<T, R: Reclaim, N: Unsigned> Internal for Owned<T, R, N> {}

use core::marker::PhantomData;
use core::ptr::NonNull;

use conquer_pointer::{MarkedNonNull, MarkedNonNullable, MarkedPtr, NonNullable};
use typenum::Unsigned;

use crate::internal::Internal;
use crate::traits::{Reclaim, SharedPointer};
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

/********** impl SharedPointer *******************************************************************/

impl<T, R: Reclaim, N: Unsigned> SharedPointer for Unprotected<T, R, N> {
    impl_shared_pointer!();
}

/********** impl MarkedNonNullable ****************************************************************/

impl<T, R, N: Unsigned> MarkedNonNullable for Unprotected<T, R, N> {
    impl_marked_non_nullable!();
}

/********** impl NonNullable **********************************************************************/

impl<T, R, N: Unsigned> NonNullable for Unprotected<T, R, N> {
    impl_non_nullable!();
}

/********** impl Internal *************************************************************************/

impl<T, R, N: Unsigned> Internal for Unprotected<T, R, N> {}

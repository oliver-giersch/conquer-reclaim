use core::marker::PhantomData;
use core::ptr::NonNull;

use conquer_pointer::{MarkedOption, NonNullable};
use typenum::Unsigned;

use crate::internal::Internal;
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

/********** impl NonNullable **********************************************************************/

impl<T, R, N: Unsigned> NonNullable for Unprotected<T, R, N> {
    impl_non_nullable!();
}

/********** impl Internal *************************************************************************/

impl<T, R, N: Unsigned> Internal for Unprotected<T, R, N> {}
impl<T, R, N: Unsigned> Internal for Option<Unprotected<T, R, N>> {}
impl<T, R, N: Unsigned> Internal for MarkedOption<Unprotected<T, R, N>> {}

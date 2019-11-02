use core::fmt;
use core::marker::PhantomData;
use core::mem;
use core::ptr::NonNull;

use conquer_pointer::{MarkedNonNull, MarkedNonNullable, MarkedPtr, NonNullable};
use typenum::Unsigned;

use crate::internal::Internal;
use crate::traits::{Reclaim, SharedPointer};
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

impl<T, R: Reclaim, N: Unsigned> Unprotected<T, R, N> {
    impl_common!();

    /// Converts the [`Unprotected`] into a ("fake" protected) [`Shared`]
    /// reference with arbitrary lifetime.
    ///
    /// # Safety
    ///
    /// The returned reference is not in fact protected and could be reclaimed
    /// by other threads, so the caller has to ensure no concurrent reclamation
    /// is possible.
    #[inline]
    pub unsafe fn into_shared<'a>(self) -> Shared<'a, T, R, N> {
        Shared { inner: self.inner, _marker: PhantomData }
    }

    /// Casts the [`Unprotected`] into a reference to a different type.
    ///
    /// # Panics
    ///
    /// This method panics if the alignment of `U` does not allow storing the
    /// required `N` mark bits.
    #[inline]
    pub fn cast<U>(unprotected: Self) -> Unprotected<U, R, N> {
        assert!(
            mem::align_of::<U>().trailing_zeros() as usize >= N::USIZE,
            "not enough available free bits in `U`"
        );
        Unprotected { inner: unprotected.inner.cast(), _marker: PhantomData }
    }
}

/********** impl Debug ****************************************************************************/

impl<T, R: Reclaim, N: Unsigned> fmt::Debug for Unprotected<T, R, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (ptr, tag) = self.inner.decompose();
        f.debug_struct("Unprotected").field("ptr", &ptr).field("tag", &tag).finish()
    }
}

/********** impl Pointer **************************************************************************/

impl<T, R: Reclaim, N: Unsigned> fmt::Pointer for Unprotected<T, R, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.inner.decompose_ptr(), f)
    }
}

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

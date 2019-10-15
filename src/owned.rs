use core::mem;
use core::ptr::NonNull;

use conquer_pointer::NonNullable;
use typenum::Unsigned;

use crate::atomic::Atomic;
use crate::record::Record;
use crate::traits::Reclaim;
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

/********** impl NonNullable **********************************************************************/

impl<T, R: Reclaim, N: Unsigned> NonNullable for Owned<T, R, N> {
    type Item = T;

    #[inline]
    fn as_const_ptr(&self) -> *const Self::Item {
        self.inner.decompose_ptr() as *const _
    }

    #[inline]
    fn as_mut_ptr(&self) -> *mut Self::Item {
        self.inner.decompose_ptr()
    }

    #[inline]
    fn as_non_null(&self) -> NonNull<Self::Item> {
        self.inner.decompose_non_null()
    }
}

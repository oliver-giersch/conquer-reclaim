use core::marker::PhantomData;
use core::ptr::NonNull;

use crate::traits::Reclaim;
use crate::Retire;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Retired
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct Retired<T, R> {
    ptr: NonNull<T>,
    _marker: PhantomData<R>,
}

/********** impl Retired **************************************************************************/

impl<T, R: Reclaim + Retire<T>> Retired<T, R> {
    // todo: as_ptr(), data_ptr(), header_ptr()

    #[inline]
    pub unsafe fn reclaim(&mut self) {
        let retired = <R as Retire<T>>::retire(self.ptr.as_ptr());
        <R as Reclaim>::reclaim(retired);
    }

    #[inline]
    pub(crate) unsafe fn new_unchecked(ptr: NonNull<T>) -> Self {
        Self { ptr, _marker: PhantomData }
    }
}

use core::ptr::NonNull;

use crate::alias::RetiredRecord;
use crate::traits::ReclaimBase;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Retired
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct Retired<R: ReclaimBase> {
    ptr: NonNull<R::Retired>,
}

/********** impl Retired **************************************************************************/

impl<R: ReclaimBase> Retired<R> {
    #[inline]
    pub fn as_ptr(&self) -> *mut () {
        self.ptr.as_ptr().cast()
    }

    #[inline]
    pub fn header_ptr(&self) -> *mut R::Header {
        unsafe { RetiredRecord::<R>::header_from_data(self.ptr.as_ptr()) }
    }

    #[inline]
    pub unsafe fn reclaim(&mut self) {
        todo!("move reclaim logic here")
        //<R as ReclaimBase>::reclaim(self.ptr.as_ptr());
    }

    #[inline]
    pub(crate) unsafe fn new_unchecked(ptr: *mut R::Retired) -> Self {
        Self { ptr: NonNull::new_unchecked(ptr) }
    }
}

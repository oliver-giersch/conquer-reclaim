use core::cmp;
use core::fmt;
use core::ptr::NonNull;

use crate::traits::ReclaimBase;

// *************************************************************************************************
// Retired
// *************************************************************************************************

pub struct Retired<R: ReclaimBase> {
    ptr: NonNull<R::Retired>,
}

/********** impl Retired **************************************************************************/

impl<R: ReclaimBase> Retired<R> {
    #[inline]
    pub fn as_ptr(&self) -> *mut () {
        unsafe { R::as_data_ptr(self.ptr.as_ptr()) }
    }

    #[inline]
    pub fn header_ptr(&self) -> *mut R::Header {
        unsafe { R::as_header_ptr(self.ptr.as_ptr()) }
    }

    #[inline]
    pub unsafe fn reclaim(&mut self) {
        R::reclaim(self.ptr.as_ptr());
    }

    #[inline]
    pub(crate) unsafe fn new_unchecked(ptr: *mut R::Retired) -> Self {
        Self { ptr: NonNull::new_unchecked(ptr) }
    }
}

/********** impl Debug ****************************************************************************/

impl<R: ReclaimBase> fmt::Debug for Retired<R> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Retired").field("addr", &self.as_ptr()).finish()
    }
}

/********** impl PartialEq ************************************************************************/

impl<R: ReclaimBase> PartialEq for Retired<R> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_ptr().eq(&other.as_ptr())
    }
}

/********** impl PartialOrd ***********************************************************************/

impl<R: ReclaimBase> PartialOrd for Retired<R> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.as_ptr().partial_cmp(&other.as_ptr())
    }
}

/********** impl Ord ******************************************************************************/

impl<R: ReclaimBase> Ord for Retired<R> {
    #[inline]
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.as_ptr().cmp(&other.as_ptr())
    }
}

/********** impl Eq *******************************************************************************/

impl<R: ReclaimBase> Eq for Retired<R> {}

/********** impl Pointer **************************************************************************/

impl<R: ReclaimBase> fmt::Pointer for Retired<R> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.as_ptr(), f)
    }
}

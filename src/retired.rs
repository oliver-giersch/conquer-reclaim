//! Type-erased (wide) pointers to retired records than can be stored and
//! later reclaimed.

use core::cmp;
use core::fmt;
use core::ptr::NonNull;

use crate::reclaim::{DropCtx, Erased};
use crate::traits::Reclaim;
use crate::AssocReclaimable;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Retired
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct Retired<R: Reclaim> {
    ptr: NonNull<AssocReclaimable<R>>,
}

/********** impl inherent *************************************************************************/

impl<R: Reclaim> Retired<R> {
    #[inline]
    pub fn as_ptr(&self) -> *mut () {
        self.ptr.cast().as_ptr()
    }

    #[inline]
    pub fn data_ptr(&self) -> *mut () {
        unsafe { R::convert_to_data(self.ptr.as_ptr()) }
    }

    #[inline]
    pub fn header_ptr(&self) -> *mut R::Header {
        unsafe { R::convert_to_header(self.ptr.as_ptr()) }
    }

    #[inline]
    pub unsafe fn reclaim(&mut self) {
        let retired = self.ptr.as_ptr();
        R::reclaim(retired);
    }

    /// Creates a new [`Retired`] from a raw non-null pointer.
    ///
    /// # Safety
    ///
    /// todo..
    #[inline]
    pub(crate) unsafe fn new_unchecked(ptr: *mut AssocReclaimable<R>) -> Self {
        Self { ptr: NonNull::new_unchecked(ptr) }
    }
}

impl<H: Default> Retired<Erased<H>> {
    #[inline]
    pub fn data(&self) -> *mut () {
        unsafe {
            let drop_ctx: NonNull<DropCtx> = self.ptr.cast();
            drop_ctx.as_ref().data
        }
    }
}

/********** impl Debug ****************************************************************************/

impl<R: Reclaim> fmt::Debug for Retired<R> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Retired")
            .field("address", &self.data_ptr())
            .field("virtual_drop", &R::VIRTUAL_DROP)
            .finish()
    }
}

/********** impl PartialEq ************************************************************************/

impl<R: Reclaim> cmp::PartialEq for Retired<R> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.ptr.eq(&other.ptr)
    }
}

/********** impl PartialOrd ***********************************************************************/

impl<R: Reclaim> cmp::PartialOrd for Retired<R> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.ptr.partial_cmp(&other.ptr)
    }
}

/********** impl Eq *******************************************************************************/

impl<R: Reclaim> cmp::Eq for Retired<R> {}

/********** impl Ord ******************************************************************************/

impl<R: Reclaim> cmp::Ord for Retired<R> {
    #[inline]
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.ptr.cmp(&other.ptr)
    }
}

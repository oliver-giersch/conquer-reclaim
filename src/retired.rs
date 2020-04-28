//! Type-erased (wide) pointers to retired records than can be stored and
//! later reclaimed.

use core::cmp;
use core::fmt;
use core::mem;
use core::ptr::NonNull;

cfg_if::cfg_if! {
    if #[cfg(feature = "std")] {
        use std::boxed::Box;
    } else {
        use alloc::boxed::Box;
    }
}

use crate::record::Record;
use crate::strategy::{DropCtx, Erased};
use crate::traits::{AssocItem, Reclaim, ReclaimStrategy};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Retired
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct Retired<R: Reclaim> {
    ptr: NonNull<AssocItem<R>>,
}

/********** impl inherent *************************************************************************/

impl<R: Reclaim> Retired<R> {
    #[inline]
    pub fn as_ptr(&self) -> *mut AssocItem<R> {
        self.ptr.as_ptr()
    }

    #[inline]
    pub fn header(&self) -> *mut R::Header {
        todo!()
    }

    #[inline]
    pub unsafe fn reclaim(&mut self) {
        let retired = self.ptr.as_ptr();
        <R::Strategy as ReclaimStrategy>::reclaim(retired);
    }

    /// Creates a new [`Retired`] from a raw non-null pointer.
    ///
    /// # Safety
    ///
    /// todo..
    #[inline]
    pub(crate) unsafe fn new_unchecked(ptr: *mut AssocItem<R>) -> Self {
        Self { ptr: NonNull::new_unchecked(ptr) }
    }
}

impl<R: Reclaim<Strategy = Erased<R>>> Retired<R> {
    pub fn data(&self) -> *mut () {
        unsafe {
            let drop_ctx: NonNull<DropCtx> = self.ptr.cast();
            drop_ctx.as_ref().data
        }
    }
}

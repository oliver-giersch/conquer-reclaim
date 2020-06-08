use core::marker::PhantomData;
use core::ptr::NonNull;

use crate::reclaim::{DynHeader, Erased, Typed};
use crate::record::Record;
use crate::traits::Reclaim;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Retired
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct Retired<T, R> {
    ptr: NonNull<T>,
    _marker: PhantomData<R>,
}

/********** impl Retired **************************************************************************/

impl<T, H> Retired<T, Typed<T, H>> {
    #[inline]
    pub fn header_ptr(&self) -> *mut H {
        unsafe { Record::<H, T>::header_from_data(self.ptr.as_ptr()) }
    }

    #[inline]
    pub fn data_ptr(&self) -> *mut T {
        self.ptr.as_ptr()
    }
}

impl<H> Retired<(), Erased<H>> {
    #[inline]
    pub fn header_ptr(&self) -> *mut DynHeader<H> {
        self.ptr.as_ptr().cast()
    }

    #[inline]
    pub fn data_ptr(&self) -> *mut () {
        let header: *mut DynHeader<H> = self.ptr.as_ptr().cast();
        unsafe { (*header).data }
    }
}

impl<R: Reclaim> Retired<R::Retired, R> {
    #[inline]
    pub unsafe fn reclaim(&mut self) {
        <R as Reclaim>::reclaim(self.ptr.as_ptr());
    }
}

impl<T, R: Reclaim> Retired<T, R> {
    #[inline]
    pub fn as_ptr(&self) -> *mut () {
        self.ptr.as_ptr().cast()
    }

    #[inline]
    pub(crate) unsafe fn new_unchecked(ptr: *mut T) -> Self {
        Self { ptr: NonNull::new_unchecked(ptr), _marker: PhantomData }
    }
}

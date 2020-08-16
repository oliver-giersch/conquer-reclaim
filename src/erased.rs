use core::any::Any;
use core::mem;
use core::ptr;

use crate::traits::ReclaimBase;

type RetiredRecord<R, T> = crate::record::Record<<R as ReclaimBase>::Header, T>;

////////////////////////////////////////////////////////////////////////////////////////////////////
// DynReclaim (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub unsafe trait DynReclaim<H>:
    ReclaimBase<Header = DynHeader<H>, Retired = DynErased>
{
    #[inline]
    unsafe fn retire<T: 'static>(ptr: *mut T) -> *mut DynErased {
        let record = RetiredRecord::<Self, T>::header_from_data(ptr);
        (*record).data_ptr = ptr as *mut dyn Any;

        record as *mut _
    }

    #[inline]
    unsafe fn reclaim(retired: *mut DynErased) {
        let header = retired as *mut DynHeader<H>;
        let record = RetiredRecord::<Self, dyn Any>::record_from_data((*header).data_ptr);
        mem::drop(Box::from(record));
    }

    #[inline]
    unsafe fn as_data_ptr(retired: *mut DynErased) -> *mut dyn Any {
        let header = retired as *mut DynHeader<H>;
        (*header).data_ptr
    }

    #[inline]
    unsafe fn as_header_ptr(retired: *mut DynErased) -> *mut Self::Header {
        retired as *mut _
    }
}

/********** blanket impl **************************************************************************/

unsafe impl<H, R> DynReclaim<H> for R where
    R: ReclaimBase<Header = DynHeader<H>, Retired = DynErased>
{
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// DynErased
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Hash)]
pub struct DynErased(());

////////////////////////////////////////////////////////////////////////////////////////////////////
// DynHeader
////////////////////////////////////////////////////////////////////////////////////////////////////

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct DynHeader<H> {
    pub(crate) data_ptr: *mut dyn Any,
    pub header: H,
}

/********** impl inherent *************************************************************************/

impl<H> DynHeader<H> {
    #[inline]
    pub fn new(header: H) -> Self {
        let null: *mut () = ptr::null_mut();
        Self { data_ptr: null as *mut dyn Any, header }
    }
}

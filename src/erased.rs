use core::any::Any;
use core::mem;
use core::ptr;

use crate::traits::ReclaimBase;

type RetiredRecord<R, T> = crate::record::Record<<R as ReclaimBase>::Header, T>;

/********** macros ********************************************************************************/

#[macro_export]
macro_rules! impl_erased_reclaim {
    ($reclaim:ty, $header:ty) => {
        unsafe impl $crate::ReclaimBase for $reclaim {
            type Header = $header;
            type Retired = $crate::DynErased;

            #[inline]
            unsafe fn reclaim(retired: *mut $crate::DynErased) {
                <Self as $crate::DynReclaim<$header>>::reclaim(retired);
            }

            #[inline(always)]
            unsafe fn as_data_ptr(retired: *mut $crate::DynErased) -> *mut dyn core::any::Any {
                <Self as $crate::DynReclaim<$header>>::as_data_ptr(retired);
            }

            #[inline(always)]
            unsafe fn as_header_ptr(retired: *mut $crate::DynErased) -> *mut $header {
                <Self as $crate::DynReclaim<$header>>::as_header_ptr(retired)
            }
        }

        unsafe impl<T: 'static> $crate::Reclaim<T> for $reclaim {
            #[inline]
            unsafe fn retire(ptr: *mut T) -> *mut $crate::DynErased {
                <Self as $crate::DynReclaim<$header>>::retire(ptr)
            }
        }
    };
}

// *************************************************************************************************
// DynReclaim (trait)
// *************************************************************************************************

pub unsafe trait DynReclaim<H: 'static>:
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

    #[inline(always)]
    unsafe fn as_data_ptr(retired: *mut DynErased) -> *mut dyn Any {
        let header = retired as *mut DynHeader<H>;
        (*header).data_ptr
    }

    #[inline(always)]
    unsafe fn as_header_ptr(retired: *mut DynErased) -> *mut Self::Header {
        retired as *mut _
    }
}

/********** blanket impl **************************************************************************/

unsafe impl<H: 'static, R> DynReclaim<H> for R where
    R: ReclaimBase<Header = DynHeader<H>, Retired = DynErased>
{
}

// *************************************************************************************************
// DynErased
// *************************************************************************************************

/// An opaque type used for type-erased pointers to records storing (dynamic)
/// reclamation information as part of a header..
#[derive(Debug, Hash)]
pub struct DynErased(());

// *************************************************************************************************
// DynHeader
// *************************************************************************************************

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

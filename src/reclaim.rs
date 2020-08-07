use core::any::Any;

use crate::alias::RetiredRecord;
use crate::traits::ReclaimBase;

////////////////////////////////////////////////////////////////////////////////////////////////////
// DynReclaim (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub unsafe trait DynReclaim<H>: ReclaimBase<Header = DynHeader<H>, Retired = dyn Any> {
    #[inline]
    unsafe fn retire<T: 'static>(ptr: *mut T) -> *mut dyn Any {
        let record = RetiredRecord::<Self>::header_from_data(ptr);
        (*record).drop_fn = |retired| {
            Box::from_raw(retired as *mut T);
        };
        ptr as _
    }

    #[inline]
    unsafe fn reclaim(retired: *mut dyn Any) {
        let header = RetiredRecord::<Self>::header_from_data(retired);
        let drop_fn = (*header).drop_fn;
        drop_fn(header as *mut ());
    }
}

/********** blanket impl **************************************************************************/

unsafe impl<H, R> DynReclaim<H> for R
where
    R: ReclaimBase<Header = DynHeader<H>, Retired = dyn Any>
{}

////////////////////////////////////////////////////////////////////////////////////////////////////
// DynHeader
////////////////////////////////////////////////////////////////////////////////////////////////////

#[repr(C)]
pub struct DynHeader<H> {
    pub(crate) drop_fn: unsafe fn(*mut ()),
    pub header: H,
}

/********** impl inherent *************************************************************************/

impl<H> DynHeader<H> {
    #[inline]
    pub fn new(header: H) -> Self {
        Self {
            drop_fn: |_| {},
            header
        }
    }
}
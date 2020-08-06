use core::any::Any;

use crate::alias::RetiredRecord;
use crate::record::Record;
use crate::traits::{Reclaim, ReclaimBase};

////////////////////////////////////////////////////////////////////////////////////////////////////
// DynReclaim (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub unsafe trait DynReclaim {
    type Header: Sized;
}

/********** impl ReclaimBase **********************************************************************/

unsafe impl<R: DynReclaim> ReclaimBase for R {
    type Header = DynHeader<R::Header>;
    type Retired = dyn Any;

    #[inline]
    unsafe fn reclaim(retired: *mut dyn Any) {
        let header = RetiredRecord::<Self>::header_from_data(retired);
        let drop_fn = (*header).drop_fn;
        drop_fn(header as *mut ());
    }
}

/********** impl Reclaim **************************************************************************/

unsafe impl<T: 'static, R: DynReclaim + ReclaimBase<Retired = dyn Any>> Reclaim<T> for R {
    #[inline]
    unsafe fn retire(ptr: *mut T) -> *mut dyn Any {
        let record: *mut Record<DynHeader<<R as DynReclaim>::Header>, T> = Record::header_from_data(ptr);
        (*record).header.drop_fn = |retired| {
            Box::from_raw(retired as *mut T);
        };
        ptr as _
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// DynHeader
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct DynHeader<H> {
    drop_fn: unsafe fn(*mut ()),
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
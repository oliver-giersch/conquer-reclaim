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
            type Retired = $crate::erased::DynErased;

            #[inline]
            unsafe fn reclaim(retired: *mut $crate::erased::DynErased) {
                <Self as $crate::erased::DynReclaim<$header>>::dyn_reclaim(retired);
            }

            #[inline(always)]
            unsafe fn as_data_ptr(
                retired: *mut $crate::erased::DynErased,
            ) -> *mut dyn core::any::Any {
                <Self as $crate::DynReclaim<$header>>::as_data_ptr(retired);
            }

            #[inline(always)]
            unsafe fn as_header_ptr(retired: *mut $crate::erased::DynErased) -> *mut $header {
                <Self as $crate::erased::DynReclaim<$header>>::as_header_ptr(retired)
            }
        }

        unsafe impl<T: 'static> $crate::Reclaim<T> for $reclaim {
            #[inline]
            unsafe fn retire(ptr: *mut T) -> *mut $crate::erased::DynErased {
                <Self as $crate::erased::DynReclaim<$header>>::dyn_retire(ptr)
            }
        }
    };
}

// *************************************************************************************************
// DynReclaim (trait)
// *************************************************************************************************

/// An extension trait for [`ReclaimBase`].
///
/// This trait is not meant to be implemented manually, since there exists a
/// blanket implementation for every type implementing [`ReclaimBase`] using
/// the appropriate associated types.
///
/// Use the [`impl_erased_reclaim`] to automatically implement these two traits
/// as well as [`Retire`] for a reclamation mechanism.
pub unsafe trait DynReclaim<H: 'static>:
    ReclaimBase<Header = DynHeader<H>, Retired = DynErased>
{
    #[inline]
    unsafe fn dyn_retire<T: 'static>(ptr: *mut T) -> *mut DynErased {
        let record = RetiredRecord::<Self, T>::header_from_data(ptr);
        (*record).data_ptr = ptr as *mut dyn Any;

        record as *mut _
    }

    #[inline]
    unsafe fn dyn_reclaim(retired: *mut DynErased) {
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

// DynReclaim is implemented automatically
unsafe impl<H: 'static, R> DynReclaim<H> for R where
    R: ReclaimBase<Header = DynHeader<H>, Retired = DynErased>
{
}

// *************************************************************************************************
// DynErased
// *************************************************************************************************

/// An opaque type used for type-erased pointers to records storing (dynamic)
/// reclamation information as part of a header.
///
/// This type is a thin wrapper over the `()` type, which functions primarily as
/// as marker, designating reclamation mechanisms, using it as their associated
/// [`Retired`][ReclaimBase::Retired] type, as *type-erased* mechanisms, being
/// able to retire and reclaim records of any type.
#[derive(Debug, Hash)]
pub struct DynErased(());

// *************************************************************************************************
// DynHeader
// *************************************************************************************************

/// A header wrapper for a type erased record.
///
/// The wrapper stores additional information required for dropping the record
/// in the record (i.e., its header) *itself*, that would normally be stored
/// alongside a pointer to the record in a *fat pointer*, such as `*mut dyn Any`.
/// Fat pointers can, however, *not* be used atomically, i.e., constructing an
/// [`AtomicPtr<dyn Any>`][core::sync::atomic::AtomicPtr] is not possible,
/// whereas a pointer to a `DynHeader<H>` *can* be used atomically..
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct DynHeader<H> {
    // NOTE: it would be sufficient and more space efficient to simply store the correct vtable
    // pointer and only construct the corresponding fat pointer when the record is reclaimed, but
    // the internal layout of fat pointers is unlikely to be stabilized soon, if ever
    pub(crate) data_ptr: *mut dyn Any,
    pub header: H,
}

/********** impl inherent *************************************************************************/

impl<H> DynHeader<H> {
    /// Wraps the given `header` in a [`DynHeader`].
    ///
    /// The resulting header wrapper is **not** yet initialized fully and can
    /// thus not be reclaimed as-is!
    /// It is necessary to always call [`retire`][DynReclaim::retire] on a
    /// pointer to the associated record's data (which is placed after the
    /// header), before calling [`reclaim`][DynReclaim::reclaim].
    #[inline]
    pub fn new(header: H) -> Self {
        let null: *mut () = ptr::null_mut();
        Self { data_ptr: null as *mut dyn Any, header }
    }
}

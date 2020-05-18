use core::mem;

use crate::traits::Reclaim;

////////////////////////////////////////////////////////////////////////////////////////////////////
// AssocRecord
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A type alias for a record associated to a [`Reclaim`] implementation.
pub(crate) type AssocRecord<T, R> = Record<<R as Reclaim>::Header, T>;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Record
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A wrapper type that prepends an instance of `T` with an instance of the
/// [`Header`][Reclaim::Header] type associated to the specified
/// [`Reclaimer`][Reclaim].
///
/// This struct guarantees the layout of its fields to match the declaration
/// order, i.e., the `header` always precedes the `data`.
#[repr(C)]
pub(crate) struct Record<H, T> {
    /// The record's header
    pub header: H,
    /// The wrapped record data itself.
    pub data: T,
}

/********** impl inherent *************************************************************************/

impl<H: Default, T> Record<H, T> {
    #[inline]
    pub fn new(data: T) -> Self {
        Self { header: H::default(), data }
    }
}

impl<H, T> Record<H, T> {
    /// Returns the pointer to the [`Record`] containing the value pointed to by
    /// `data`.
    ///
    /// # Safety
    ///
    /// The `data` pointer must be a valid pointer to an instance of `T` that
    /// was constructed as part of a [`Record`].
    #[inline]
    pub unsafe fn ptr_from_data(data: *mut T) -> *mut Self {
        (data as *mut u8).sub(Self::data_offset()).cast()
    }

    /// Returns the pointer to the [`header`][Record::header] field of the
    /// [`Record`] containing the value pointed to by `data`.
    ///
    /// # Safety
    ///
    /// The `data` pointer must be a valid pointer to an instance of `T` that
    /// was constructed as part of a [`Record`].
    #[inline]
    pub unsafe fn header_from_data(data: *mut T) -> *mut H {
        (data as *mut u8).sub(Self::header_offset()).cast()
    }

    #[inline]
    const fn header_offset() -> usize {
        0
    }

    #[inline]
    const fn data_offset() -> usize {
        let offset = Self::header_offset() + mem::size_of::<H>();
        offset + offset.wrapping_neg() % mem::align_of::<T>()
    }
}

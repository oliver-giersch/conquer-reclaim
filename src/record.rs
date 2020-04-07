use core::mem;

use crate::traits::Reclaim;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Record
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A wrapper type that prepends an instance of `T` with an instance of the
/// [`Header`][Reclaim::Header] type associated to the specified
/// [`Reclaimer`][Reclaim].
///
/// This struct guarantees the layout of its fields to match the declaration
/// order, i.e., the `header` always precedes the `data`.
#[derive(Debug, Default, Copy, Clone, Eq, Ord, PartialEq, PartialOrd)]
#[repr(C)]
pub struct Record<T, R: Reclaim> {
    /// The record's header
    pub(crate) header: R::Header,
    /// The wrapped record data itself.
    pub(crate) data: T,
}

/********** impl inherent *************************************************************************/

impl<T, R: Reclaim> Record<T, R> {
    /// Creates a new [`Record`] with the specified `data` and a default header.
    #[inline]
    pub fn new(data: T) -> Self {
        Self { header: Default::default(), data }
    }

    /// Creates a new [`Record`] with the specified `header` and `data`.
    #[inline]
    pub fn with_header(header: R::Header, data: T) -> Self {
        Self { header, data }
    }

    /// Returns the pointer to the [`Record`] containing the value pointed to by
    /// `data`.
    ///
    /// # Safety
    ///
    /// The `data` pointer must be a valid pointer to an instance of `T` that
    /// was constructed as part of a [`Record`].
    #[inline]
    pub unsafe fn ptr_from_data(data: *mut T) -> *mut Self {
        ((data as usize) - Self::offset_data()) as *mut _
    }

    /// Returns the pointer to the [`header`][Record::header] field of the
    /// [`Record`] containing the value pointed to by the `data`.
    ///
    /// # Safety
    ///
    /// The `data` pointer must be a valid pointer to an instance of `T` that
    /// was constructed as part of a [`Record`].
    #[inline]
    pub unsafe fn header_ptr_from_data(data: *mut T) -> *mut R::Header {
        Self::ptr_from_data(data).cast()
    }

    /// Returns the offset in bytes from the address of a [`Record`] to its
    /// [`data`][Record::data] field.
    #[inline]
    pub fn offset_data() -> usize {
        record_header_to_data_offset::<R::Header, T>()
    }
}

/********** helper function ***********************************************************************/

#[inline]
const fn record_header_to_data_offset<H, T>() -> usize {
    // this matches rustc's algorithm for laying out #[repr(C)] types.
    let header_size = mem::size_of::<H>();
    let data_align = mem::align_of::<T>();

    header_size + (header_size.wrapping_neg() & (data_align - 1))
}

use core::mem;

use crate::traits::Reclaim;

////////////////////////////////////////////////////////////////////////////////////////////////////
// AssocRecord
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A type alias for a record associated to a [`Reclaim`] implementation.
pub(crate) type AssocRecord<T, R> = Record<<R as Reclaim>::DropCtx, <R as Reclaim>::Header, T>;

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
pub(crate) struct Record<C, H, T> {
    /// The record's drop context.
    pub drop_ctx: C,
    /// The record's header
    pub header: H,
    /// The wrapped record data itself.
    pub data: T,
}

/********** impl inherent *************************************************************************/

impl<C: Default, H: Default, T> Record<C, H, T> {
    #[inline]
    pub fn new(data: T) -> Self {
        Self { drop_ctx: C::default(), header: H::default(), data }
    }
}

impl<C, H, T> Record<C, H, T> {
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
    const fn drop_ctx_offset() -> usize {
        0
    }

    #[inline]
    const fn header_offset() -> usize {
        let offset = Self::drop_ctx_offset() + mem::size_of::<C>();
        offset + offset.wrapping_neg() % mem::align_of::<H>()
    }

    #[inline]
    const fn data_offset() -> usize {
        let offset = Self::header_offset() + mem::size_of::<H>();
        offset + offset.wrapping_neg() % mem::align_of::<T>()
    }
}

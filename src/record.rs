use core::mem;
use core::ptr::NonNull;

use memoffset::offset_of;

use crate::traits::Reclaimer;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Record
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A wrapper type for `T` that is associated with a concrete reclamation and
/// contains additional data per heap-allocated value.
///
/// Each time a new [`Owned`][crate::Owned] or (non-null)
/// [`Atomic`][crate::atomic::Atomic] is created, an instance of this type is
/// allocated as a wrapper for the desired value.
/// The record and its header are never directly exposed to the data structure
/// using a given memory reclamation scheme and should only be accessed by the
/// reclamation scheme itself.
#[derive(Debug, Default, Copy, Clone, Eq, Ord, PartialEq, PartialOrd)]
pub struct Record<T, R: Reclaimer> {
    /// The record's header
    pub(crate) header: R::Header,
    /// The record's wrapped (inner) element
    pub(crate) elem: T,
}

/********** impl inherent *************************************************************************/

impl<T, R: Reclaimer> Record<T, R> {
    /// Creates a new record with the specified `elem` and a default header.
    #[inline]
    pub fn new(elem: T) -> Self {
        Self { header: Default::default(), elem }
    }

    /// Creates a new record with the specified `elem` and `header`.
    #[inline]
    pub fn with_header(elem: T, header: R::Header) -> Self {
        Self { header, elem }
    }

    /// Returns a reference to the record's header.
    #[inline]
    pub fn header(&self) -> &R::Header {
        &self.header
    }

    /// Returns a reference to the record's element.
    #[inline]
    pub fn elem(&self) -> &T {
        &self.elem
    }

    /// Calculates the address of the [`Record`] for the given pointer to a
    /// wrapped non-nullable `elem` and returns the resulting pointer.
    ///
    /// # Safety
    ///
    /// The `elem` pointer must be a valid pointer to an instance of `T` that
    /// was constructed as part of a [`Record`]. Otherwise, the pointer
    /// arithmetic used to determine the address will result in a pointer to
    /// unrelated memory, which is likely to lead to undefined behaviour.
    #[inline]
    pub unsafe fn from_raw_non_null(elem: NonNull<T>) -> NonNull<Self> {
        Self::from_raw(elem.as_ptr())
    }

    /// Calculates the address of the [`Record`] for the given pointer to a
    /// wrapped `elem` and returns the resulting pointer.
    ///
    /// # Safety
    ///
    /// The `elem` pointer must be a valid pointer to an instance of `T` that
    /// was constructed as part of a [`Record`]. Otherwise, the pointer
    /// arithmetic used to determine the address will result in a pointer to
    /// unrelated memory, which is likely to lead to undefined behaviour.
    #[inline]
    pub unsafe fn from_raw(elem: *mut T) -> NonNull<Self> {
        let addr = (elem as usize) - Self::offset_elem();
        NonNull::new_unchecked(addr as *mut _)
    }

    /// Returns a reference to the header for the record at the pointed-to
    /// location of the pointer `elem`.
    ///
    /// # Safety
    ///
    /// The pointer `elem` must be a valid pointer to an instance of `T` that
    /// was allocated as part of a `Record`.
    /// Otherwise, the pointer arithmetic used to calculate the header's address
    /// will be incorrect and lead to undefined behavior.
    #[inline]
    pub unsafe fn header_from_raw<'a>(elem: *mut T) -> &'a R::Header {
        let header = (elem as usize) - Self::offset_elem() + Self::offset_header();
        &*(header as *mut _)
    }

    /// Returns a reference to the header for the record at the pointed-to
    /// location of the non-nullable pointer `elem`.
    ///
    /// # Safety
    ///
    /// The pointer `elem` must be a valid pointer to an instance of `T` that
    /// was allocated as part of a `Record`.
    /// Otherwise, the pointer arithmetic used to calculate the header's address
    /// will be incorrect and lead to undefined behavior.
    #[inline]
    pub unsafe fn header_from_raw_non_null<'a>(elem: NonNull<T>) -> &'a R::Header {
        let header = (elem.as_ptr() as usize) - Self::offset_elem() + Self::offset_header();
        &*(header as *mut _)
    }

    /// Returns the offset in bytes from the address of a record to its header
    /// field.
    #[inline]
    pub fn offset_header() -> usize {
        if mem::size_of::<R::Header>() == 0 {
            0
        } else {
            // FIXME:
            //  the offset_of! macro is unsound, replace once a sound alternative becomes available
            //  https://internals.rust-lang.org/t/pre-rfc-add-a-new-offset-of-macro-to-core-mem/9273
            offset_of!(Self, header)
        }
    }

    /// Returns the offset in bytes from the address of a record to its element
    /// field.
    #[inline]
    pub fn offset_elem() -> usize {
        if mem::size_of::<R::Header>() == 0 {
            0
        } else {
            // FIXME:
            //  the offset_of! macro is unsound, replace once a sound alternative becomes available
            //  https://internals.rust-lang.org/t/pre-rfc-add-a-new-offset-of-macro-to-core-mem/9273
            offset_of!(Self, elem)
        }
    }
}

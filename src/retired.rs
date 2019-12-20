//! Type-erased (wide) pointers to retired records than can be stored and
//! later reclaimed.

use core::cmp;
use core::fmt;
use core::marker::PhantomData;
use core::mem;
use core::ptr::NonNull;

cfg_if::cfg_if! {
    if #[cfg(feature = "std")] {
        use std::boxed::Box;
    } else {
        use alloc::boxed::Box;
    }
}

use crate::record::Record;
use crate::traits::Reclaim;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Retired
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct Retired<R> {
    raw: RawRetired,
    _marker: PhantomData<R>,
}

/********** impl inherent *************************************************************************/

impl<R: Reclaim + 'static> Retired<R> {
    /// Creates a new [`Retired`] record from a raw pointer.
    ///
    /// # Safety
    ///
    /// The caller has to ensure numerous safety invariants in order for a
    /// [`Retired`] record to be used safely:
    ///
    /// - the given `record` pointer **must** point to a valid heap allocated
    ///   value
    /// - the record **must** have been allocated as part of a [`Record`] of
    ///   the appropriate [`Reclaim`] implementation
    /// - *if* the type of the retired record implements [`Drop`] *and* contains
    ///   any non-static references, it must be ensured that these are **not**
    ///   accessed by the [`drop`][Drop::drop] function.
    #[inline]
    pub(crate) unsafe fn new<'a, T: 'a>(ptr: NonNull<T>) -> Self {
        let any: NonNull<dyn Any + 'a> = Record::<T, R>::from_raw_non_null(ptr);
        let any: NonNull<dyn Any + 'static> = mem::transmute(any);

        Self { raw: RawRetired { ptr: any }, _marker: PhantomData }
    }

    #[inline]
    pub fn into_raw(self) -> RawRetired {
        self.raw
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// RawRetired
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A type-erased fat pointer to a retired record.
pub struct RawRetired {
    ptr: NonNull<dyn Any + 'static>,
}

/********** impl inherent *************************************************************************/

impl RawRetired {
    /// Converts a retired record to a raw pointer.
    ///
    /// Since retired records are type-erased trait object (fat) pointers to
    /// retired values that should no longer be used, only the 'address' part
    /// of the pointer is returned, i.e. a pointer to an `()`.
    #[inline]
    pub fn as_ptr(&self) -> *const () {
        self.ptr.as_ptr() as *mut () as *const ()
    }

    /// Returns the numeric representation of the retired record's memory
    /// address.
    #[inline]
    pub fn address(&self) -> usize {
        self.ptr.as_ptr() as *mut () as usize
    }

    /// Reclaims the retired record by dropping it and de-allocating its memory.
    ///
    /// # Safety
    ///
    /// This method **must** not be called more than once or when some other
    /// thread or scope still has some reference to the record.
    #[inline]
    pub unsafe fn reclaim(&mut self) {
        mem::drop(Box::from_raw(self.ptr.as_ptr()));
    }
}

/********** impl PartialEq ************************************************************************/

impl PartialEq for RawRetired {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_ptr().eq(&other.as_ptr())
    }
}

/********** impl PartialOrd ***********************************************************************/

impl PartialOrd for RawRetired {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.as_ptr().partial_cmp(&other.as_ptr())
    }
}

/********** impl Ord ******************************************************************************/

impl Ord for RawRetired {
    #[inline]
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.as_ptr().cmp(&other.as_ptr())
    }
}

/********** impl Eq *******************************************************************************/

impl Eq for RawRetired {}

/********** impl Debug ****************************************************************************/

impl fmt::Debug for RawRetired {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Retired").field("address", &self.as_ptr()).finish()
    }
}

/********** impl Display **************************************************************************/

impl fmt::Display for RawRetired {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.as_ptr(), f)
    }
}

/********** impl Pointer **************************************************************************/

impl fmt::Pointer for RawRetired {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.as_ptr(), f)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Any (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

trait Any {}
impl<T> Any for T {}

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
use crate::traits::Reclaimer;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Retired
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A type-erased fat pointer to a retired record.
pub struct Retired<R>(NonNull<dyn Any + 'static>, PhantomData<R>);

/********** impl inherent *************************************************************************/

impl<R: Reclaimer + 'static> Retired<R> {
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
    pub fn new<'a, T: 'a>(record: NonNull<T>) -> Self {
        unsafe {
            let any: NonNull<dyn Any + 'a> = Record::<T, R>::from_raw_non_null(record);
            let any: NonNull<dyn Any + 'static> = mem::transmute(any);

            Self(any, PhantomData)
        }
    }

    /// Converts a retired record to a raw pointer.
    ///
    /// Since retired records are type-erased trait object (fat) pointers to
    /// retired values that should no longer be used, only the 'address' part
    /// of the pointer is returned, i.e. a pointer to an `()`.
    #[inline]
    pub fn as_ptr(&self) -> *const () {
        self.0.as_ptr() as *mut () as *const ()
    }

    /// Returns the numeric representation of the retired record's memory
    /// address.
    #[inline]
    pub fn address(&self) -> usize {
        self.0.as_ptr() as *mut () as usize
    }

    /// Reclaims the retired record by dropping it and de-allocating its memory.
    ///
    /// # Safety
    ///
    /// This method **must** not be called more than once or when some other
    /// thread or scope still has some reference to the record.
    #[inline]
    pub unsafe fn reclaim(&mut self) {
        mem::drop(Box::from_raw(self.0.as_ptr()));
    }
}

/********** impl PartialEq ************************************************************************/

impl<R: Reclaimer + 'static> PartialEq for Retired<R> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_ptr().eq(&other.as_ptr())
    }
}

/********** impl PartialOrd ***********************************************************************/

impl<R: Reclaimer + 'static> PartialOrd for Retired<R> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.as_ptr().partial_cmp(&other.as_ptr())
    }
}

/********** impl Ord ******************************************************************************/

impl<R: Reclaimer + 'static> Ord for Retired<R> {
    #[inline]
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.as_ptr().cmp(&other.as_ptr())
    }
}

/********** impl Eq *******************************************************************************/

impl<R: Reclaimer + 'static> Eq for Retired<R> {}

/********** impl Debug ****************************************************************************/

impl<R: Reclaimer + 'static> fmt::Debug for Retired<R> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Retired").field("address", &self.as_ptr()).finish()
    }
}

/********** impl Display **************************************************************************/

impl<R: Reclaimer + 'static> fmt::Display for Retired<R> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.as_ptr(), f)
    }
}

/********** impl Pointer **************************************************************************/

impl<R: Reclaimer + 'static> fmt::Pointer for Retired<R> {
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

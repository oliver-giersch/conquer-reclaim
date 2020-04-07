//! Type-erased (wide) pointers to retired records than can be stored and
//! later reclaimed.

use core::alloc::Layout;
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
    raw: RetiredPtr,
    _marker: PhantomData<R>,
}

/********** impl inherent *************************************************************************/

impl<R: Reclaim + 'static> Retired<R> {
    /// Returns the raw pointer to the retired record.
    #[inline]
    pub fn into_raw(self) -> RetiredPtr {
        self.raw
    }

    /// Creates a new [`Retired`] from a raw non-`null` pointer.
    ///
    /// # Safety
    ///
    /// todo..
    #[inline]
    pub(crate) unsafe fn new_unchecked<'a, T: 'a>(ptr: NonNull<T>) -> Self {
        let record = Record::<T, R>::ptr_from_data(ptr.as_ptr());
        let any: NonNull<dyn Any> = NonNull::new_unchecked(record);

        Self { raw: RetiredPtr { ptr: mem::transmute(any) }, _marker: PhantomData }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// RetiredPtr
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A type-erased, non-`null` fat pointer to a retired record.
pub struct RetiredPtr {
    ptr: NonNull<dyn Any + 'static>,
}

/********** impl inherent *************************************************************************/

impl RetiredPtr {
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

    #[inline]
    pub fn layout(&self) -> Layout {
        Layout::for_value(unsafe { self.ptr.as_ref() })
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

impl PartialEq for RetiredPtr {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_ptr().eq(&other.as_ptr())
    }
}

/********** impl PartialOrd ***********************************************************************/

impl PartialOrd for RetiredPtr {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.as_ptr().partial_cmp(&other.as_ptr())
    }
}

/********** impl Ord ******************************************************************************/

impl Ord for RetiredPtr {
    #[inline]
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.as_ptr().cmp(&other.as_ptr())
    }
}

/********** impl Eq *******************************************************************************/

impl Eq for RetiredPtr {}

/********** impl Debug ****************************************************************************/

impl fmt::Debug for RetiredPtr {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Retired").field("address", &self.as_ptr()).finish()
    }
}

/********** impl Pointer **************************************************************************/

impl fmt::Pointer for RetiredPtr {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.as_ptr(), f)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Any (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A trait like [`Any`][core::any::Any] but without the `'static` bound that
/// does not allow down-casting and is mainly useful for ensuring that the
/// correct `Drop` code is run when dropping an `Box<dyn Any>`.
trait Any {}

/********** blanket impl for all types ************************************************************/

impl<T> Any for T {}

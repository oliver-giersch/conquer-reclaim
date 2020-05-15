use core::borrow::{Borrow, BorrowMut};
use core::convert::{AsMut, AsRef};
use core::fmt;
use core::marker::PhantomData;
use core::mem::{self, ManuallyDrop};
use core::ops::{Deref, DerefMut};
use core::ptr::NonNull;

cfg_if::cfg_if! {
    if #[cfg(feature = "std")] {
        use std::boxed::Box;
    } else {
        use alloc::boxed::Box;
    }
}

use conquer_pointer::typenum::Unsigned;
use conquer_pointer::{MarkedNonNull, MarkedPtr};

use crate::atomic::Storable;
use crate::record::AssocRecord;
use crate::traits::Reclaim;
use crate::Owned;

/********** impl Clone ****************************************************************************/

impl<T: Clone, R: Reclaim, N: Unsigned> Clone for Owned<T, R, N> {
    #[inline]
    fn clone(&self) -> Self {
        let (reference, tag) = unsafe { self.inner.decompose_ref() };
        Self::with_tag(reference.clone(), tag)
    }
}

/********** impl Send + Sync **********************************************************************/

unsafe impl<T, R: Reclaim, N: Unsigned> Send for Owned<T, R, N> where T: Send {}
unsafe impl<T, R: Reclaim, N: Unsigned> Sync for Owned<T, R, N> where T: Sync {}

/********** impl inherent *************************************************************************/

impl<T, R: Reclaim, N: Unsigned> Owned<T, R, N> {
    /// Creates a new heap-allocated [`Record<T>`](Record) and returns an owning
    /// handle to it.
    #[inline]
    pub fn new(owned: T) -> Self {
        Self {
            inner: unsafe { MarkedNonNull::compose_unchecked(Self::alloc_record(owned), 0) },
            _marker: PhantomData,
        }
    }

    /// Creates a new `Owned` like [`new`](Owned::new) but composes the
    /// returned pointer with an initial `tag` value.
    ///
    /// # Example
    ///
    /// The primary use case for this is to pre-mark newly allocated values.
    ///
    /// ```
    /// use core::sync::atomic::Ordering;
    ///
    /// use reclaim::typenum::U1;
    /// use reclaim::Shared;
    ///
    /// type Atomic<T> = reclaim::leak::Atomic<T, U1>;
    /// type Owned<T> = reclaim::leak::Owned<T, U1>;
    ///
    /// let atomic = Atomic::null();
    /// let owned = Owned::with_tag("string", 0b1);
    ///
    /// atomic.store(owned, Ordering::Relaxed);
    /// let shared = atomic.load_shared(Ordering::Relaxed);
    ///
    /// assert_eq!((&"string", 0b1), shared.unwrap().decompose_ref());
    /// ```
    #[inline]
    pub fn with_tag(owned: T, tag: usize) -> Self {
        Self {
            inner: unsafe { MarkedNonNull::compose_unchecked(Self::alloc_record(owned), tag) },
            _marker: PhantomData,
        }
    }

    impl_from_ptr!();
    impl_from_non_null!();

    /// Consumes the [`Owned`], de-allocates its memory and extracts the
    /// contained value.
    ///
    /// This has the same semantics as destructuring a [`Box`].
    #[inline]
    #[allow(clippy::wrong_self_convention)]
    pub fn into_inner(owned: Self) -> T {
        let boxed: Box<AssocRecord<_, R>> = owned.into();
        (*boxed).data
    }

    #[inline]
    #[allow(clippy::wrong_self_convention)]
    pub fn into_marked_ptr(owned: Self) -> MarkedPtr<T, N> {
        let owned = ManuallyDrop::new(owned);
        owned.inner.into_marked_ptr()
    }

    #[inline]
    #[allow(clippy::wrong_self_convention)]
    pub fn into_marked_non_null(owned: Self) -> MarkedNonNull<T, N> {
        let owned = ManuallyDrop::new(owned);
        owned.inner
    }

    #[inline]
    #[allow(clippy::wrong_self_convention)]
    pub fn as_marked_ptr(owned: &Self) -> MarkedPtr<T, N> {
        owned.inner.into_marked_ptr()
    }

    #[inline]
    pub fn as_marked_non_null(owned: &Self) -> MarkedNonNull<T, N> {
        owned.inner
    }

    #[inline]
    pub fn clear_tag(owned: Self) -> Self {
        let owned = ManuallyDrop::new(owned);
        Self { inner: owned.inner.clear_tag(), _marker: PhantomData }
    }

    #[inline]
    pub fn set_tag(owned: Self, tag: usize) -> Self {
        let owned = ManuallyDrop::new(owned);
        Self { inner: owned.inner.set_tag(tag), _marker: PhantomData }
    }

    #[inline]
    pub fn decompose_tag(owned: &Self) -> usize {
        owned.inner.decompose_tag()
    }

    /// Decomposes the internal marked pointer, returning a reference and the
    /// separated tag.
    ///
    /// # Example
    ///
    /// ```
    /// use core::sync::atomic::Ordering::Relaxed;
    ///
    /// use conquer_reclaim::typenum::U1;
    /// use conquer_reclaim::leak::Owned;
    ///
    /// type Atomic<T> = reclaim::leak::Atomic<T, U1>;
    ///
    /// let mut atomic = Atomic::from(Owned::with_tag("string", 0b1));
    /// // ... potential operations by other threads ...
    /// let owned = atomic.take(); // after all threads have joined
    ///
    /// assert_eq!((&"string", 0b1), Owned::decompose_ref(owned.as_ref().unwrap()));
    /// ```
    #[inline]
    pub fn decompose_ref(owned: &Self) -> (&T, usize) {
        // this is safe because is `inner` is guaranteed to be backed by a valid allocation
        unsafe { owned.inner.decompose_ref() }
    }

    /// Decomposes the internal marked pointer, returning a mutable reference
    /// and the separated tag.
    #[inline]
    pub fn decompose_mut(owned: &mut Self) -> (&mut T, usize) {
        // this is safe because is `inner` is guaranteed to be backed by a valid allocation
        unsafe { owned.inner.decompose_mut() }
    }

    #[inline]
    pub fn leak(owned: Self) -> Storable<T, R, N> {
        let owned = ManuallyDrop::new(owned);
        Storable::new(owned.inner.into())
    }

    #[inline]
    pub unsafe fn from_storable(storable: Storable<T, R, N>) -> Self {
        Self {
            inner: MarkedNonNull::new_unchecked(storable.into_marked_ptr()),
            _marker: PhantomData,
        }
    }

    /// Allocates a records wrapping `owned` and returns the pointer to the
    /// wrapped value.
    #[inline]
    fn alloc_record(owned: T) -> NonNull<T> {
        let record = Box::leak(Box::new(AssocRecord::<_, R>::new(owned)));
        NonNull::from(&record.data)
    }
}

/********** impl AsRef ****************************************************************************/

impl<T, R: Reclaim, N: Unsigned> AsRef<T> for Owned<T, R, N> {
    #[inline]
    fn as_ref(&self) -> &T {
        self.deref()
    }
}

/********** impl AsMut ****************************************************************************/

impl<T, R: Reclaim, N: Unsigned> AsMut<T> for Owned<T, R, N> {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        self.deref_mut()
    }
}

/********** impl Borrow ***************************************************************************/

impl<T, R: Reclaim, N: Unsigned> Borrow<T> for Owned<T, R, N> {
    #[inline]
    fn borrow(&self) -> &T {
        self.deref()
    }
}

/********** impl BorrowMut ************************************************************************/

impl<T, R: Reclaim, N: Unsigned> BorrowMut<T> for Owned<T, R, N> {
    #[inline]
    fn borrow_mut(&mut self) -> &mut T {
        self.deref_mut()
    }
}

/********** impl Debug ****************************************************************************/

impl<T, R: Reclaim, N: Unsigned> fmt::Debug for Owned<T, R, N>
where
    T: fmt::Debug,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (reference, tag) = unsafe { self.inner.decompose_ref() };
        f.debug_struct("Owned").field("value", reference).field("tag", &tag).finish()
    }
}

/********** impl Default **************************************************************************/

impl<T: Default, R: Reclaim, N: Unsigned> Default for Owned<T, R, N> {
    #[inline]
    fn default() -> Self {
        Self::new(T::default())
    }
}

/********** impl Deref ****************************************************************************/

impl<T, R: Reclaim, N: Unsigned> Deref for Owned<T, R, N> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { self.inner.as_ref() }
    }
}

/********** impl DerefMut *************************************************************************/

impl<T, R: Reclaim, N: Unsigned> DerefMut for Owned<T, R, N> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.inner.as_mut() }
    }
}

/********** impl Pointer **************************************************************************/

impl<T, R: Reclaim, N: Unsigned> fmt::Pointer for Owned<T, R, N> {
    impl_fmt_pointer!();
}

/********** impl Drop *****************************************************************************/

impl<T, R: Reclaim, N: Unsigned> Drop for Owned<T, R, N> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            let record = AssocRecord::<_, R>::ptr_from_data(self.inner.decompose_ptr());
            mem::drop(Box::from_raw(record));
        }
    }
}

/********** impl From (T) *************************************************************************/

impl<T, R: Reclaim, N: Unsigned> From<T> for Owned<T, R, N> {
    #[inline]
    fn from(owned: T) -> Self {
        Self::new(owned)
    }
}

/********** impl From (Owned) for Box<Record<T, R>> ***********************************************/

impl<T, R: Reclaim, N: Unsigned> From<Owned<T, R, N>> for Box<AssocRecord<T, R>> {
    #[inline]
    fn from(owned: Owned<T, R, N>) -> Self {
        unsafe {
            let record = AssocRecord::<_, R>::ptr_from_data(owned.inner.decompose_ptr());
            Box::from_raw(record)
        }
    }
}

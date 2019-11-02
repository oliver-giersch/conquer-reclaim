use core::borrow::{Borrow, BorrowMut};
use core::convert::{AsMut, AsRef, TryFrom};
use core::fmt;
use core::marker::PhantomData;
use core::mem;
use core::ptr::NonNull;

#[cfg(not(feature = "std"))]
use alloc::boxed::Box;
#[cfg(feature = "std")]
use std::boxed::Box;

use conquer_pointer::{MarkedNonNull, MarkedNonNullable, MarkedPtr, NonNullable, NullError};
use typenum::Unsigned;

use crate::atomic::Atomic;
use crate::internal::Internal;
use crate::record::Record;
use crate::traits::{Reclaim, SharedPointer};
use crate::{Owned, Shared, Unprotected};

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
    /// Allocates memory for a [`Record<T>`](Record) on the heap and then
    /// places a record with a default header and `owned` into it.
    ///
    /// This does only allocate memory if at least one of
    /// [`RecordHeader`][header] or `T` are not zero-sized.
    /// If the [`RecordHeader`][header] is a ZST, this behaves
    /// identically to `Box::new`.
    ///
    /// [header]: crate::LocalReclaim::RecordHeader
    #[inline]
    pub fn new(owned: T) -> Self {
        let record = Self::alloc_record(owned);
        Self {
            inner: unsafe { MarkedNonNull::from_non_null_unchecked(record) },
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
    /// assert_eq!((&"string", 0b1), Shared::decompose_ref(shared.unwrap()));
    /// ```
    #[inline]
    pub fn with_tag(owned: T, tag: usize) -> Self {
        Self {
            inner: unsafe { MarkedNonNull::compose_unchecked(Self::alloc_record(owned), tag) },
            _marker: PhantomData,
        }
    }

    /// Consumes the [`Owned`], de-allocates its memory and extracts the
    /// contained value.
    ///
    /// This has the same semantics as destructuring a [`Box`].
    #[inline]
    pub fn into_inner(self) -> T {
        unsafe {
            let ptr = self.inner.decompose_ptr();
            mem::forget(self);
            let boxed = Box::from_raw(Record::<_, R>::from_raw(ptr).as_ptr());
            (*boxed).elem
        }
    }

    impl_common!();

    /// Decomposes the internal marked pointer, returning a reference and the
    /// separated tag.
    ///
    /// # Example
    ///
    /// ```
    /// use core::sync::atomic::Ordering::Relaxed;
    ///
    /// use reclaim::typenum::U1;
    /// use reclaim::leak::Owned;
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

    /// Consumes and leaks the `Owned`, returning a mutable reference
    /// `&'a mut T` and the decomposed tag.
    /// Note that the type `T` must outlive the chosen lifetime `'a`.
    /// If the type has only static references, or none at all, then this may
    /// chosen to be `'static`.
    #[inline]
    pub fn leak<'a>(owned: Self) -> (&'a mut T, usize)
    where
        T: 'a,
    {
        let (ptr, tag) = owned.inner.decompose();
        mem::forget(owned);
        unsafe { (&mut *ptr.as_ptr(), tag) }
    }

    /// Leaks the `owned` value and turns it into an [`Unprotected`] value,
    /// which has copy semantics, but can no longer be safely dereferenced.
    ///
    /// # Example
    ///
    /// ```
    /// use core::sync::atomic::Ordering::Relaxed;
    ///
    /// use reclaim::typenum::U0;
    /// use reclaim::{Owned, Shared};
    ///
    /// type Atomic<T> = reclaim::leak::Atomic<T, U0>;
    ///
    /// let atomic = Atomic::null();
    ///
    /// let unprotected = Owned::leak_unprotected(Owned::new("string"));
    ///
    /// loop {
    ///     // `unprotected` is simply copied in every loop iteration
    ///     if atomic.compare_exchange_weak(Shared::none(), unprotected, Relaxed, Relaxed).is_ok() {
    ///         break;
    ///     }
    /// }
    ///
    /// # assert_eq!(&"string", &*atomic.load_shared(Relaxed).unwrap())
    /// ```
    #[inline]
    pub fn leak_unprotected(owned: Self) -> Unprotected<T, R, N> {
        let inner = owned.inner;
        mem::forget(owned);
        Unprotected { inner, _marker: PhantomData }
    }

    /// Leaks the `owned` value and turns it into a "protected" [`Shared`][shared]
    /// value with arbitrary lifetime `'a`.
    ///
    /// Note, that the protection of the [`Shared`][shared] value in this case
    /// stems from the fact, that the given `owned` could not have previously
    /// been part of a concurrent data structure (barring unsafe construction).
    /// This rules out concurrent reclamation by other threads.
    ///
    /// # Safety
    ///
    /// Once a leaked [`Shared`][shared] has been successfully inserted into a
    /// concurrent data structure, it must not be accessed any more, if there is
    /// the possibility for concurrent reclamation of the record.
    ///
    /// [shared]: crate::Shared
    ///
    /// # Example
    ///
    /// The use case for this method is similar to [`leak_unprotected`][Owned::leak_unprotected]
    /// but the leaked value can be safely dereferenced **before** being
    /// inserted into a shared data structure.
    ///
    /// ```
    /// use core::sync::atomic::Ordering::Relaxed;
    ///
    /// use reclaim::typenum::U0;
    /// use reclaim::{Owned, Shared};
    ///
    /// type Atomic<T> = reclaim::leak::Atomic<T, U0>;
    ///
    /// let atomic = Atomic::null();
    ///
    /// let shared = unsafe {
    ///     Owned::leak_shared(Owned::new("string"))
    /// };
    ///
    /// assert_eq!(&"string", &*shared);
    ///
    /// loop {
    ///     // `shared` is simply copied in every loop iteration
    ///     if atomic.compare_exchange_weak(Shared::none(), shared, Relaxed, Relaxed).is_ok() {
    ///         // if (non-leaking) reclamation is going on, `shared` must not be accessed
    ///         // anymore after successful insertion!
    ///         break;
    ///     }
    /// }
    ///
    /// # assert_eq!(&"string", &*atomic.load_shared(Relaxed).unwrap())
    /// ```
    #[inline]
    pub unsafe fn leak_shared<'a>(owned: Self) -> Shared<'a, T, R, N> {
        let inner = owned.inner;
        mem::forget(owned);
        Shared { inner, _marker: PhantomData }
    }

    /// Allocates a records wrapping `owned` and returns the pointer to the
    /// wrapped value.
    #[inline]
    fn alloc_record(owned: T) -> NonNull<T> {
        let record = Box::leak(Box::new(Record::<_, R>::new(owned)));
        NonNull::from(&record.elem)
    }

    #[inline]
    fn deref(&self) -> &T {
        unsafe { self.inner.as_ref() }
    }

    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        unsafe { self.inner.as_mut() }
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

/********** impl Default **************************************************************************/

impl<T: Default, R: Reclaim, N: Unsigned> Default for Owned<T, R, N> {
    #[inline]
    fn default() -> Self {
        Self::new(T::default())
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

/********** impl Pointer **************************************************************************/

impl<T, R: Reclaim, N: Unsigned> fmt::Pointer for Owned<T, R, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.inner.decompose_ptr(), f)
    }
}

/********** impl Drop *****************************************************************************/

impl<T, R: Reclaim, N: Unsigned> Drop for Owned<T, R, N> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            let record = Record::<_, R>::from_raw(self.inner.decompose_ptr());
            mem::drop(Box::from_raw(record.as_ptr()));
        }
    }
}

/********** impl From *****************************************************************************/

impl<T, R: Reclaim, N: Unsigned> From<T> for Owned<T, R, N> {
    #[inline]
    fn from(owned: T) -> Self {
        Self::new(owned)
    }
}

/********** impl TryFrom **************************************************************************/

impl<T, R: Reclaim, N: Unsigned> TryFrom<Atomic<T, R, N>> for Owned<T, R, N> {
    type Error = NullError;

    #[inline]
    fn try_from(atomic: Atomic<T, R, N>) -> Result<Self, Self::Error> {
        atomic.into_owned().ok_or(NullError)
    }
}

/********** impl SharedPointer *******************************************************************/

impl<T, R: Reclaim, N: Unsigned> SharedPointer for Owned<T, R, N> {
    impl_shared_pointer!();
}

/********** impl MarkedNonNullable ****************************************************************/

impl<T, R: Reclaim, N: Unsigned> MarkedNonNullable for Owned<T, R, N> {
    impl_marked_non_nullable!();
}

/********** impl NonNullable **********************************************************************/

impl<T, R: Reclaim, N: Unsigned> NonNullable for Owned<T, R, N> {
    impl_non_nullable!();
}

/********** impl Internal *************************************************************************/

impl<T, R: Reclaim, N: Unsigned> Internal for Owned<T, R, N> {}

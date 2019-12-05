use core::borrow::{Borrow, BorrowMut};
use core::convert::{AsMut, AsRef, TryFrom};
use core::fmt;
use core::marker::PhantomData;
use core::mem;
use core::ops::{Deref, DerefMut};
use core::ptr::NonNull;

#[cfg(not(feature = "std"))]
use alloc::boxed::Box;
#[cfg(feature = "std")]
use std::boxed::Box;

use conquer_pointer::{MarkedNonNull, MarkedNonNullable, MarkedPtr, NonNullable, NullError};
use typenum::Unsigned;

use crate::atomic::Atomic;
use crate::record::Record;
use crate::traits::Reclaimer;
use crate::{Owned, Shared};

/********** impl Clone ****************************************************************************/

impl<T: Clone, R: Reclaimer, N: Unsigned> Clone for Owned<T, R, N> {
    #[inline]
    fn clone(&self) -> Self {
        let (reference, tag) = unsafe { self.inner.decompose_ref() };
        Self::with_tag(reference.clone(), tag)
    }
}

/********** impl Send + Sync **********************************************************************/

unsafe impl<T, R: Reclaimer, N: Unsigned> Send for Owned<T, R, N> where T: Send {}
unsafe impl<T, R: Reclaimer, N: Unsigned> Sync for Owned<T, R, N> where T: Sync {}

/********** impl inherent *************************************************************************/

impl<T, R: Reclaimer, N: Unsigned> Owned<T, R, N> {
    /// Creates a new heap-allocated [`Record<T>`](Record) and returns an owning
    /// handle to it.
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
    /// assert_eq!((&"string", 0b1), shared.unwrap().decompose_ref());
    /// ```
    #[inline]
    pub fn with_tag(owned: T, tag: usize) -> Self {
        Self {
            inner: unsafe { MarkedNonNull::compose_unchecked(Self::alloc_record(owned), tag) },
            _marker: PhantomData,
        }
    }

    impl_common_from!();

    /// Consumes the [`Owned`], de-allocates its memory and extracts the
    /// contained value.
    ///
    /// This has the same semantics as destructuring a [`Box`].
    #[inline]
    #[allow(clippy::wrong_self_convention)]
    pub fn into_inner(owned: Self) -> T {
        unsafe {
            let ptr = owned.inner.decompose_ptr();
            mem::forget(owned);
            let boxed = Box::from_raw(Record::<_, R>::from_raw(ptr).as_ptr());
            (*boxed).elem
        }
    }

    #[inline]
    #[allow(clippy::wrong_self_convention)]
    pub fn into_marked_ptr(owned: Self) -> MarkedPtr<T, N> {
        let ptr = owned.inner.into_marked_ptr();
        mem::forget(owned);
        ptr
    }

    #[inline]
    #[allow(clippy::wrong_self_convention)]
    pub fn into_marked_non_null(owned: Self) -> MarkedNonNull<T, N> {
        let ptr = owned.inner;
        mem::forget(owned);
        ptr
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
    pub fn tag(owned: &Self) -> usize {
        owned.inner.decompose_tag()
    }

    #[inline]
    pub fn set_tag(owned: Self, tag: usize) -> Self {
        let inner = owned.inner.set_tag(tag);
        mem::forget(owned);

        Self { inner, _marker: PhantomData }
    }

    #[inline]
    pub fn split_tag(owned: Self) -> (Self, usize) {
        let (inner, tag) = owned.inner.split_tag();
        mem::forget(owned);

        (Self { inner, _marker: PhantomData }, tag)
    }

    #[inline]
    pub fn clear_tag(owned: Self) -> Self {
        let inner = owned.inner.clear_tag();
        mem::forget(owned);

        Self { inner, _marker: PhantomData }
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
    /// use conquer_reclaim::typenum::U0;
    /// use conquer_reclaim::{Owned, Shared};
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
}

/********** impl AsRef ****************************************************************************/

impl<T, R: Reclaimer, N: Unsigned> AsRef<T> for Owned<T, R, N> {
    #[inline]
    fn as_ref(&self) -> &T {
        self.deref()
    }
}

/********** impl AsMut ****************************************************************************/

impl<T, R: Reclaimer, N: Unsigned> AsMut<T> for Owned<T, R, N> {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        self.deref_mut()
    }
}

/********** impl Borrow ***************************************************************************/

impl<T, R: Reclaimer, N: Unsigned> Borrow<T> for Owned<T, R, N> {
    #[inline]
    fn borrow(&self) -> &T {
        self.deref()
    }
}

/********** impl BorrowMut ************************************************************************/

impl<T, R: Reclaimer, N: Unsigned> BorrowMut<T> for Owned<T, R, N> {
    #[inline]
    fn borrow_mut(&mut self) -> &mut T {
        self.deref_mut()
    }
}

/********** impl Default **************************************************************************/

impl<T: Default, R: Reclaimer, N: Unsigned> Default for Owned<T, R, N> {
    #[inline]
    fn default() -> Self {
        Self::new(T::default())
    }
}

/********** impl Debug ****************************************************************************/

impl<T, R: Reclaimer, N: Unsigned> fmt::Debug for Owned<T, R, N>
where
    T: fmt::Debug,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (reference, tag) = unsafe { self.inner.decompose_ref() };
        f.debug_struct("Owned").field("value", reference).field("tag", &tag).finish()
    }
}

/********** impl Deref ****************************************************************************/

impl<T, R: Reclaimer, N: Unsigned> Deref for Owned<T, R, N> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { self.inner.as_ref() }
    }
}

/********** impl DerefMut *************************************************************************/

impl<T, R: Reclaimer, N: Unsigned> DerefMut for Owned<T, R, N> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.inner.as_mut() }
    }
}

/********** impl Pointer **************************************************************************/

impl<T, R: Reclaimer, N: Unsigned> fmt::Pointer for Owned<T, R, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.inner.decompose_ptr(), f)
    }
}

/********** impl Drop *****************************************************************************/

impl<T, R: Reclaimer, N: Unsigned> Drop for Owned<T, R, N> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            let record = Record::<_, R>::from_raw(self.inner.decompose_ptr());
            mem::drop(Box::from_raw(record.as_ptr()));
        }
    }
}

/********** impl From *****************************************************************************/

impl<T, R: Reclaimer, N: Unsigned> From<T> for Owned<T, R, N> {
    #[inline]
    fn from(owned: T) -> Self {
        Self::new(owned)
    }
}

/********** impl TryFrom **************************************************************************/

impl<T, R: Reclaimer, N: Unsigned> TryFrom<Atomic<T, R, N>> for Owned<T, R, N> {
    type Error = NullError;

    #[inline]
    fn try_from(atomic: Atomic<T, R, N>) -> Result<Self, Self::Error> {
        atomic.into_owned().ok_or(NullError)
    }
}

/********** impl MarkedNonNullable ****************************************************************/

impl<T, R: Reclaimer, N: Unsigned> MarkedNonNullable for Owned<T, R, N> {
    impl_marked_non_nullable!();
}

/********** impl NonNullable **********************************************************************/

impl<T, R: Reclaimer, N: Unsigned> NonNullable for Owned<T, R, N> {
    impl_non_nullable!();
}

mod compare;
mod store;

use core::fmt;
use core::marker::PhantomData;
use core::mem::ManuallyDrop;
use core::ptr;
use core::sync::atomic::Ordering;

use conquer_pointer::typenum::Unsigned;
use conquer_pointer::{AtomicMarkedPtr, MarkedNonNull, MarkedPtr, Null};

use crate::traits::{Protect, Reclaim};
use crate::{Maybe, NotEqual, Owned, Protected, Unlinked, Unprotected};

pub use self::compare::Comparable;
pub use self::store::Storable;

use self::compare::Unlink;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Atomic
////////////////////////////////////////////////////////////////////////////////////////////////////

/// An atomic marked pointer type to a heap allocated value similar to
/// [`AtomicPtr`](core::sync::atomic::AtomicPtr).
///
/// Note, that the type does not implement the [`Drop`][core::ops::Drop] trait,
/// meaning it does not automatically take care of memory de-allocation when it
/// goes out of scope.
/// Use the (unsafe) [`take`][Atomic::take] method to extract an (optional)
/// [`Owned`] value, which *does* correctly deallocate memory when it goes out
/// of scope.
pub struct Atomic<T, R, N> {
    inner: AtomicMarkedPtr<T, N>,
    _marker: PhantomData<(T, R)>,
}

/********** impl inherent (const) *****************************************************************/

impl<T, R, N> Atomic<T, R, N> {
    /// Creates a new `null` pointer.
    #[inline]
    pub const fn null() -> Self {
        Self { inner: AtomicMarkedPtr::null(), _marker: PhantomData }
    }

    /// Returns a reference to the underlying (raw) [`AtomicMarkedPtr`].
    ///
    /// # Safety
    ///
    /// The returned reference to the raw pointer must not be used to store
    /// invalid values into the [`Atomic`].
    #[inline]
    pub const unsafe fn as_raw(&self) -> &AtomicMarkedPtr<T, N> {
        &self.inner
    }
}

/********** impl inherent *************************************************************************/

impl<T, R: Reclaim, N: Unsigned> Atomic<T, R, N> {
    /// Allocates a new [`Record`][crate::Record] for `val` and stores an
    /// [`Atomic`] pointer to it.
    #[inline]
    pub fn new(val: T) -> Self {
        Self::from(Owned::from(val))
    }

    /// Creates a new [`Atomic`] from the given raw `ptr`.
    ///
    /// # Safety
    ///
    /// The given `ptr` argument must be a pointer to a valid heap allocated
    /// instance of `T` that was allocated as part of a
    /// [`Record`][rec], e.g., through an [`Owned`].
    ///
    /// Note, that creating more than one [`Atomic`] from the same
    /// [`Record`][rec] has implications for other methods such as
    /// [`take`][Atomic::take].
    ///
    /// [rec]: crate::Record
    #[inline]
    pub unsafe fn from_raw(ptr: MarkedPtr<T, N>) -> Self {
        Self { inner: AtomicMarkedPtr::new(ptr), _marker: PhantomData }
    }

    /// TODO: docs...
    #[inline]
    pub unsafe fn take(&mut self) -> Option<Owned<T, R, N>> {
        match MarkedNonNull::new(self.inner.swap(MarkedPtr::null(), Ordering::Relaxed)) {
            Ok(inner) => Some(Owned { inner, _marker: PhantomData }),
            Err(_) => None,
        }
    }

    /// Loads a raw marked pointer from the [`Atomic`].
    ///
    /// `load_raw` takes an [`Ordering`] argument, which describes the
    /// memory ordering of this operation.
    ///
    /// # Panics
    ///
    /// Panics if `order` is [`Release`][Ordering::Release] or
    /// [`AcqRel`][Ordering::AcqRel].
    ///
    /// # Example
    ///
    /// Commonly, this is likely going to be used in conjunction with
    /// [`load_if_equal`][Atomic::load_if_equal] or
    /// [`acquire_if_equal`][Protect::acquire_if_equal].
    ///
    /// ```
    /// use std::sync::atomic::Ordering::Relaxed;
    ///
    /// use reclaim::typenum::U0;
    /// use reclaim::leak::Guard;
    ///
    /// type Atomic<T> = reclaim::leak::Atomic<T, U0>;
    ///
    /// let atomic = Atomic::new("string");
    /// let guard = &Guard::new();
    ///
    /// let ptr = atomic.load_raw(Relaxed);
    /// let res = atomic.load_if_equal(ptr, Relaxed, guard);
    ///
    /// assert!(res.is_ok());
    /// # assert_eq!(&"string", &*res.unwrap().unwrap());
    /// ```
    #[inline]
    pub fn load_raw(&self, order: Ordering) -> MarkedPtr<T, N> {
        self.inner.load(order)
    }

    /// Loads an [`Unprotected`] pointer from the [`Atomic`].
    ///
    /// The returned pointer is explicitly **not** protected from reclamation,
    /// meaning another thread could free the pointed to memory at any time.
    ///
    /// `load_unprotected` takes an [`Ordering`] argument, which
    /// describes the memory ordering of this operation.
    ///
    /// # Panics
    ///
    /// Panics if `order` is [`Release`][rel] or [`AcqRel`][acq_rel].
    ///
    /// [acq_rel]: Ordering::AcqRel
    /// [rel]: Ordering::Release
    #[inline]
    pub fn load_unprotected(&self, order: Ordering) -> Unprotected<T, R, N> {
        Unprotected { inner: self.load_raw(order), _marker: PhantomData }
    }

    /// Loads a value from the pointer using `guard` to protect it from
    /// reclamation.
    ///
    /// If the loaded value is non-null, the value is guaranteed to be protected
    /// from reclamation during the lifetime of `guard`.
    ///
    /// `load` takes an [`Ordering`][ordering] argument, which describes the
    /// memory ordering of this operation.
    ///
    /// # Panics
    ///
    /// *May* panic if `order` is [`Release`][release] or [`AcqRel`][acq_rel].
    ///
    /// [release]: Ordering::Release
    /// [acq_rel]: Ordering::AcqRel
    #[inline]
    pub fn load<'g>(&self, guard: &'g mut R::Guard, order: Ordering) -> Protected<'g, T, R, N> {
        guard.protect(self, order)
    }

    /// TODO: docs...
    #[inline]
    pub fn load_if_equal<'g>(
        &self,
        expected: MarkedPtr<T, N>,
        guard: &'g mut R::Guard,
        order: Ordering,
    ) -> Result<Protected<'g, T, R, N>, NotEqual> {
        guard.protect_if_equal(self, expected, order)
    }

    /// Stores either `null` or a valid pointer to an owned heap allocated value
    /// into the pointer.
    ///
    /// Note, that overwriting a non-null value through `store` will very likely
    /// lead to memory leaks, since instances of [`Atomic`] will most commonly
    /// be associated wit some kind of uniqueness invariants in order to be sound.
    ///
    /// `store` takes an [`Ordering`][ordering] argument, which
    /// describes the memory ordering of this operation.
    ///
    /// # Panics
    ///
    /// Panics if `order` is [`Acquire`][acquire] or [`AcqRel`][acq_rel]
    ///
    /// [ordering]: Ordering
    /// [acquire]: Ordering::Acquire
    /// [acq_rel]: Ordering::AcqRel
    #[inline]
    pub fn store(&self, new: impl Into<Storable<T, R, N>>, order: Ordering) {
        self.inner.store(new.into().into_marked_ptr(), order);
    }

    /// Stores either `null` or a valid pointer to an owned heap allocated value
    /// into the pointer, returning the previous (now [`Unlinked`]) value
    /// wrapped in an [`Option`].
    ///
    /// The returned value can be safely reclaimed as long as the *uniqueness*
    /// invariant is maintained.
    ///
    /// `swap` takes an [`Ordering`][ordering] argument which describes the memory
    /// ordering of this operation. All ordering modes are possible. Note that using
    /// [`Acquire`][acquire] makes the store part of this operation [`Relaxed`][relaxed],
    /// and using [`Release`][release] makes the load part [`Relaxed`][relaxed].
    ///
    /// [ordering]: Ordering
    /// [relaxed]: Ordering::Relaxed
    /// [acquire]: Ordering::Acquire
    /// [release]: Ordering::Release
    #[inline]
    pub fn swap(
        &self,
        new: impl Into<Storable<T, R, N>>,
        order: Ordering,
    ) -> Maybe<Unlinked<T, R, N>> {
        match MarkedNonNull::new(self.inner.swap(new.into().into_marked_ptr(), order)) {
            Ok(inner) => Maybe::Some(Unlinked { inner, _marker: PhantomData }),
            Err(Null(tag)) => Maybe::Null(tag),
        }
    }

    #[inline]
    pub fn compare_exchange<C, S>(
        &self,
        current: C,
        new: S,
        (success, failure): (Ordering, Ordering),
    ) -> Result<C::Unlinked, CompareExchangeErr<S, T, R, N>>
    where
        C: Into<Comparable<T, R, N>> + Unlink + Copy,
        S: Into<Storable<T, R, N>>,
    {
        let new = ManuallyDrop::new(new);
        unsafe {
            let compare = current.into().into_marked_ptr();
            let store = ptr::read(&*new).into().into_marked_ptr();
            self.inner
                .compare_exchange(compare, store, (success, failure))
                .map(|_| current.into_unlinked())
                .map_err(|inner| CompareExchangeErr {
                    loaded: Unprotected { inner, _marker: PhantomData },
                    input: ManuallyDrop::into_inner(new),
                })
        }
    }

    #[inline]
    pub fn compare_exchange_weak<C, S>(
        &self,
        current: C,
        new: S,
        (success, failure): (Ordering, Ordering),
    ) -> Result<C::Unlinked, CompareExchangeErr<S, T, R, N>>
    where
        C: Into<Comparable<T, R, N>> + Unlink + Copy,
        S: Into<Storable<T, R, N>>,
    {
        let new = ManuallyDrop::new(new);
        unsafe {
            let compare = current.into().into_marked_ptr();
            let store = ptr::read(&*new).into().into_marked_ptr();
            self.inner
                .compare_exchange_weak(compare, store, (success, failure))
                .map(|_| current.into_unlinked())
                .map_err(|inner| CompareExchangeErr {
                    loaded: Unprotected { inner, _marker: PhantomData },
                    input: ManuallyDrop::into_inner(new),
                })
        }
    }

    #[inline]
    pub(crate) fn load_raw_if_equal(
        &self,
        expected: MarkedPtr<T, N>,
        order: Ordering,
    ) -> Result<MarkedPtr<T, N>, NotEqual> {
        match self.load_raw(order) {
            ptr if ptr == expected => Ok(ptr),
            _ => Err(NotEqual),
        }
    }
}

/********** impl Default **************************************************************************/

impl<T, R: Reclaim, N: Unsigned> Default for Atomic<T, R, N> {
    #[inline]
    fn default() -> Self {
        Self::null()
    }
}

/********** impl Debug ****************************************************************************/

impl<T, R, N: Unsigned> fmt::Debug for Atomic<T, R, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (ptr, tag) = self.inner.load(Ordering::SeqCst).decompose();
        f.debug_struct("Atomic").field("ptr", &ptr).field("tag", &tag).finish()
    }
}

/********** impl From (T) *************************************************************************/

impl<T, R: Reclaim, N: Unsigned> From<T> for Atomic<T, R, N> {
    #[inline]
    fn from(val: T) -> Self {
        Self::from(Owned::new(val))
    }
}

/********** impl From (Owned<T>) ******************************************************************/

impl<T, R: Reclaim, N: Unsigned> From<Owned<T, R, N>> for Atomic<T, R, N> {
    #[inline]
    fn from(owned: Owned<T, R, N>) -> Self {
        Self { inner: AtomicMarkedPtr::from(Owned::into_marked_ptr(owned)), _marker: PhantomData }
    }
}

/********** impl From (Storable) ******************************************************************/

impl<T, R: Reclaim, N: Unsigned> From<Storable<T, R, N>> for Atomic<T, R, N> {
    #[inline]
    fn from(storable: Storable<T, R, N>) -> Self {
        Self { inner: AtomicMarkedPtr::new(storable.into_marked_ptr()), _marker: PhantomData }
    }
}

/********** impl Pointer **************************************************************************/

impl<T, R: Reclaim, N: Unsigned> fmt::Pointer for Atomic<T, R, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.inner.load(Ordering::SeqCst), f)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// CompareExchangeErr
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Copy, Clone, Eq, Ord, PartialEq, PartialOrd)]
pub struct CompareExchangeErr<S, T, R, N> {
    pub loaded: Unprotected<T, R, N>,
    pub input: S,
}

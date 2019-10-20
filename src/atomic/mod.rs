mod compare;
mod guard;

use core::fmt;
use core::marker::PhantomData;
use core::mem::ManuallyDrop;
use core::sync::atomic::Ordering;

use conquer_pointer::{AtomicMarkedPtr, MarkedOption, MarkedPtr};
use typenum::Unsigned;

pub use self::compare::CompareAndSwap;
pub use self::guard::GuardRef;

use crate::traits::{Reclaim, SharedPointer};
use crate::{Owned, Shared, Unlinked, Unprotected};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Atomic
////////////////////////////////////////////////////////////////////////////////////////////////////

/// An atomic marked pointer type to an owned heap allocated value similar to
/// [`AtomicPtr`](core::sync::atomic::AtomicPtr).
///
/// The `Atomic` type has similarities to [`Option<Box>`][Option], as it is a
/// pointer that is either `null` or otherwise must point to a valid, heap
/// allocated value.
/// Note, that the type does not implement the [`Drop`](core::ops::Drop) trait,
/// meaning it does not automatically take care of memory de-allocation when it
/// goes out of scope.
/// Use the [`take`][Atomic::take] method to extract an (optional) [`Owned`]
/// value, which *does* correctly deallocate memory when it goes out of scope.
pub struct Atomic<T, R, N> {
    inner: AtomicMarkedPtr<T, N>,
    _marker: PhantomData<(T, R)>,
}

/********** impl Send + Sync **********************************************************************/

unsafe impl<T, R: Reclaim, N: Unsigned> Send for Atomic<T, R, N> where T: Send + Sync {}
unsafe impl<T, R: Reclaim, N: Unsigned> Sync for Atomic<T, R, N> where T: Send + Sync {}

/********** impl inherent (const) *****************************************************************/

impl<T, R, N> Atomic<T, R, N> {
    /// Creates a new `null` pointer.
    #[inline]
    pub const fn null() -> Self {
        Self { inner: AtomicMarkedPtr::null(), _marker: PhantomData }
    }

    /// Gets a reference to the underlying (raw) [`AtomicMarkedPtr`].
    #[inline]
    pub const fn as_raw(&self) -> &AtomicMarkedPtr<T, N> {
        &self.inner
    }
}

/********** impl inherent *************************************************************************/

impl<T, R: Reclaim, N: Unsigned> Atomic<T, R, N> {
    /// Allocates a new [`Owned`] containing the given `val` and immediately
    /// storing it an `Atomic`.
    #[inline]
    pub fn new(val: T) -> Self {
        Self::from(Owned::from(val))
    }

    /// TODO: Docs...
    #[inline]
    pub fn get_mut(&mut self) -> Option<&mut T> {
        unsafe { self.inner.load(Ordering::Relaxed).as_mut() }
    }

    /// TODO: Docs...    
    #[inline]
    pub fn into_owned(self) -> Option<Owned<T, R, N>> {
        MarkedOption::from(self.inner.load(Ordering::Relaxed))
            .map(|ptr| Owned { inner: ptr, _marker: PhantomData })
            .value()
    }

    /// Creates a new [`Atomic`] from the given `ptr`.
    ///
    /// # Safety
    ///
    /// The given `ptr` argument must be a pointer to a valid heap allocated
    /// instance of `T` that was allocated as part of a [`Record`][crate::Record],
    /// e.g. through an [`Owned`].
    /// The same pointer should also not be used to create more than one
    /// [`Atomic`]s.
    #[inline]
    pub unsafe fn from_raw(ptr: MarkedPtr<T, N>) -> Self {
        Self { inner: AtomicMarkedPtr::new(ptr), _marker: PhantomData }
    }

    /// Loads a raw marked value from the pointer.
    ///
    /// `load_raw` takes an [`Ordering`][ordering] argument, which describes the
    /// memory ordering of this operation.
    ///
    /// # Panics
    ///
    /// Panics if `order` is [`Release`][release] or [`AcqRel`][acq_rel].
    ///
    /// [ordering]: core::sync::atomic::Ordering
    /// [release]: core::sync::atomic::Ordering::Release
    /// [acq_rel]: core::sync::atomic::Ordering::AcqRel
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

    /// Loads an [`Unprotected`] reference wrapped in a [`MarkedOption`] from
    /// the `Atomic`.
    ///
    /// The returned reference is explicitly **not** protected from reclamation,
    /// meaning another thread could free the value's memory at any time.
    ///
    /// This method is similar to [`load_raw`][Atomic::load_raw], but the
    /// resulting [`Unprotected`] type has stronger guarantees than a raw
    /// [`MarkedPtr`].
    /// It can be useful to load an unprotected pointer if that pointer does not
    /// need to be de-referenced, but is only used to reinsert it in a different
    /// spot, which is e.g. done when removing a value from a linked list.
    ///
    /// # Panics
    ///
    /// Panics if `order` is [`Release`][release] or [`AcqRel`][acq_rel].
    ///
    /// [ordering]: core::sync::atomic::Ordering
    /// [release]: core::sync::atomic::Ordering::Release
    /// [acq_rel]: core::sync::atomic::Ordering::AcqRel
    #[inline]
    pub fn load_unprotected_marked_option(
        &self,
        order: Ordering,
    ) -> MarkedOption<Unprotected<T, R, N>> {
        MarkedOption::from(self.inner.load(order))
            .map(|ptr| Unprotected { inner: ptr, _marker: PhantomData })
    }

    /// Loads an optional [`Unprotected`] reference from the `Atomic`.
    ///
    /// The returned reference is explicitly **not** protected from reclamation,
    /// meaning another thread could free the value's memory at any time.
    ///
    /// This method is similar to [`load_raw`][Atomic::load_raw], but the
    /// resulting [`Unprotected`] type has stronger guarantees than a raw
    /// [`MarkedPtr`].
    /// It can be useful to load an unprotected pointer if that pointer does not
    /// need to be de-referenced, but is only used to reinsert it in a different
    /// spot, which is e.g. done when removing a value from a linked list.
    ///
    /// # Panics
    ///
    /// Panics if `order` is [`Release`][release] or [`AcqRel`][acq_rel].
    ///
    /// [ordering]: core::sync::atomic::Ordering
    /// [release]: core::sync::atomic::Ordering::Release
    /// [acq_rel]: core::sync::atomic::Ordering::AcqRel
    #[inline]
    pub fn load_unprotected(&self, order: Ordering) -> Option<Unprotected<T, R, N>> {
        self.load_unprotected_marked_option(order).value()
    }

    /// Loads a value from the pointer and uses `guard` to protect it.
    /// The (optional) protected [`Shared`] value is wrapped in a [`Marked].
    ///
    /// If the loaded value is non-null, the value is guaranteed to be protected
    /// from reclamation during the lifetime of `guard`.
    ///
    /// The primary difference to [`load`][Atomic::load] is, that the returned
    /// [`Marked`] type is additionally able to represent marked `null`
    /// pointers.
    ///
    /// `load_marked` takes an [`Ordering`][ordering] argument, which describes
    /// the memory ordering of this operation.
    ///
    /// # Panics
    ///
    /// *May* panic if `order` is [`Release`][release] or [`AcqRel`][acq_rel].
    ///
    /// [ordering]: core::sync::atomic::Ordering
    /// [release]: core::sync::atomic::Ordering::Release
    /// [acq_rel]: core::sync::atomic::Ordering::AcqRel
    #[inline]
    pub fn load_marked_option<'g>(
        &self,
        guard: impl GuardRef<'g, Reclaimer = R>,
        order: Ordering,
    ) -> MarkedOption<Shared<'g, T, R, N>> {
        guard.load_protected(self, order)
    }

    /// Loads a value from the pointer and uses `guard` to protect it.
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
    /// [ordering]: Ordering
    /// [release]: Ordering::Release
    /// [acq_rel]: Ordering::AcqRel
    #[inline]
    pub fn load<'g>(
        &self,
        guard: impl GuardRef<'g, Reclaimer = R>,
        order: Ordering,
    ) -> Option<Shared<'g, T, R, N>> {
        self.load_marked_option(guard, order).value()
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
    pub fn store(
        &self,
        ptr: impl SharedPointer<Item = T, MarkBits = N, Reclaimer = R>,
        order: Ordering,
    ) {
        self.inner.store(ptr.into_marked_ptr(), order);
    }

    /// Stores either `null` or a valid pointer to an owned heap allocated value
    /// into the pointer, returning the previous (now [`Unlinked`]) value
    /// wrapped in a [`MarkedOption`].
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
    pub fn swap_marked_option(
        &self,
        ptr: impl SharedPointer<Item = T, Reclaimer = R, MarkBits = N>,
        order: Ordering,
    ) -> MarkedOption<Unlinked<T, R, N>> {
        let prev = self.inner.swap(ptr.into_marked_ptr(), order);
        MarkedOption::from(prev).map(|ptr| Unlinked { inner: ptr, _marker: PhantomData })
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
        ptr: impl SharedPointer<Item = T, Reclaimer = R, MarkBits = N>,
        order: Ordering,
    ) -> Option<Unlinked<T, R, N>> {
        self.swap_marked_option(ptr, order).value()
    }

    #[inline]
    pub fn compare_exchange<C, I>(
        &self,
        current: C,
        new: I,
        success: Ordering,
        failure: Ordering,
    ) -> Result<C::Success, CompareExchangeError<C, I>>
    where
        C: CompareAndSwap<Item = T, Reclaimer = R, MarkBits = N>,
        I: SharedPointer<Item = T, Reclaimer = R, MarkBits = N>,
    {
        let new = ManuallyDrop::new(new);
        self.inner
            .compare_exchange(current.into_marked_ptr(), new.as_marked_ptr(), success, failure)
            .map(|ptr| unsafe { <C::Success as SharedPointer>::from_marked_ptr(ptr) })
            .map_err(|ptr| CompareExchangeError {
                loaded: unsafe { <C::Failure as SharedPointer>::from_marked_ptr(ptr) },
                input: ManuallyDrop::into_inner(new),
                _private: (),
            })
    }

    #[inline]
    pub fn compare_exchange_weak<C, I>(
        &self,
        current: C,
        new: I,
        success: Ordering,
        failure: Ordering,
    ) -> Result<C::Success, CompareExchangeError<C, I>>
    where
        C: CompareAndSwap<Item = T, Reclaimer = R, MarkBits = N>,
        I: SharedPointer<Item = T, Reclaimer = R, MarkBits = N>,
    {
        let new = ManuallyDrop::new(new);
        self.inner
            .compare_exchange_weak(current.into_marked_ptr(), new.as_marked_ptr(), success, failure)
            .map(|ptr| unsafe { <C::Success as SharedPointer>::from_marked_ptr(ptr) })
            .map_err(|ptr| CompareExchangeError {
                loaded: unsafe { <C::Failure as SharedPointer>::from_marked_ptr(ptr) },
                input: ManuallyDrop::into_inner(new),
                _private: (),
            })
    }

    #[inline]
    pub fn take(&mut self) -> Option<Owned<T, R, N>> {
        MarkedOption::from(self.inner.swap(MarkedPtr::null(), Ordering::Relaxed))
            .map(|ptr| Owned { inner: ptr, _marker: PhantomData })
            .value()
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

impl<T, R: Reclaim, N: Unsigned> fmt::Debug for Atomic<T, R, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (ptr, tag) = self.inner.load(Ordering::SeqCst).decompose();
        f.debug_struct("Atomic").field("ptr", &ptr).field("tag", &tag).finish()
    }
}

/********** impl From *****************************************************************************/

impl<T, R: Reclaim, N: Unsigned> From<Owned<T, R, N>> for Atomic<T, R, N> {
    #[inline]
    fn from(owned: Owned<T, R, N>) -> Self {
        Self { inner: AtomicMarkedPtr::from(Owned::into_marked_ptr(owned)), _marker: PhantomData }
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
// CompareExchangeError
////////////////////////////////////////////////////////////////////////////////////////////////////

/// The returned error type for a failed [`compare_exchange`](Atomic::compare_exchange) or
/// [`compare_exchange_weak`](Atomic::compare_exchange_weak) operation.
#[derive(Copy, Clone, Debug)]
pub struct CompareExchangeError<C, I>
where
    C: CompareAndSwap,
    I: SharedPointer,
{
    /// The actually loaded value
    pub loaded: C::Failure,
    /// The value with which the failed swap was attempted
    pub input: I,
    // prevents construction outside of the current module
    _private: (),
}

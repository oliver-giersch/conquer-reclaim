use core::fmt;
use core::iter::{FromIterator, IntoIterator};
use core::mem::{self, ManuallyDrop};
use core::ptr;
use core::sync::atomic::Ordering::{self, Acquire, Relaxed, Release};

#[cfg(feature = "examples-debug")]
use core::sync::atomic::AtomicUsize;

cfg_if::cfg_if! {
    if #[cfg(feature = "std")] {
        use std::sync::Arc;
    } else {
        use alloc::sync::Arc;
    }
}

use crate::conquer_pointer::MarkedPtr;
use crate::{Maybe, ReclaimRef, ReclaimThreadState};

type Atomic<T, R> = crate::Atomic<T, R, 0>;
type Owned<T, R> = crate::Owned<T, R, 0>;

/********** global node drop counter (debug) ******************************************************/

#[cfg(feature = "examples-debug")]
pub static NODE_DROP_COUNTER: AtomicUsize = AtomicUsize::new(0);

////////////////////////////////////////////////////////////////////////////////////////////////////
// ArcStack
////////////////////////////////////////////////////////////////////////////////////////////////////

/// An [`Arc`] based version of Treiber's lock-free stack.
pub struct ArcStack<T, R: ReclaimRef<Node<T, R>>> {
    inner: Arc<Stack<T, R>>,
    thread_state: ManuallyDrop<R::ThreadState>,
}

/*********** impl Send ****************************************************************************/

unsafe impl<T, R: ReclaimRef<Node<T, R>>> Send for ArcStack<T, R> {}

/*********** impl Clone ***************************************************************************/

impl<T, R: ReclaimRef<Node<T, R>>> Clone for ArcStack<T, R> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            thread_state: ManuallyDrop::new(unsafe {
                self.inner.reclaimer.build_thread_state_unchecked()
            }),
        }
    }
}

/*********** impl inherent ************************************************************************/

impl<T, R: ReclaimRef<Node<T, R>> + Default> ArcStack<T, R> {
    #[inline]
    pub fn new() -> Self {
        Self::with_reclaimer(Default::default())
    }
}

impl<T, R: ReclaimRef<Node<T, R>>> ArcStack<T, R> {
    #[inline]
    pub fn with_reclaimer(reclaimer: R) -> Self {
        let inner = Arc::new(Stack::<_, R>::with_reclaimer(reclaimer));
        let thread_state = unsafe { inner.reclaimer.build_thread_state_unchecked() };
        Self { inner, thread_state: ManuallyDrop::new(thread_state) }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    #[inline]
    pub fn push(&self, elem: T) {
        unsafe { self.inner.push(elem, &self.thread_state) }
    }

    #[inline]
    pub fn pop(&self) -> Option<T> {
        unsafe { self.inner.pop_unchecked(&self.thread_state) }
    }

    #[inline]
    pub fn try_unwrap(self) -> Result<Stack<T, R>, Self> {
        // circumvents the restrictions on moving out of types implementing Drop.
        let (inner, mut thread_state) = unsafe {
            let inner = ptr::read(&self.inner);
            let thread_state = ptr::read(&self.thread_state);
            mem::forget(self);
            (inner, thread_state)
        };

        Arc::try_unwrap(inner)
            .map(|stack| {
                unsafe { ManuallyDrop::drop(&mut thread_state) };
                stack
            })
            .map_err(|inner| Self { inner, thread_state })
    }
}

/********** impl Debug ****************************************************************************/

impl<T, R: ReclaimRef<Node<T, R>> + Default> fmt::Debug for ArcStack<T, R> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ArcStack {{ ... }}")
    }
}

/********** impl Default **************************************************************************/

impl<T, R: ReclaimRef<Node<T, R>> + Default> Default for ArcStack<T, R> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/********** impl Drop *****************************************************************************/

impl<T, R: ReclaimRef<Node<T, R>>> Drop for ArcStack<T, R> {
    #[inline]
    fn drop(&mut self) {
        // SAFETY: Drop Local state before the `Arc`, because it may hold a pointer into it.
        unsafe { ManuallyDrop::drop(&mut self.thread_state) };
    }
}

/********** impl From (Stack) *********************************************************************/

impl<T, R: ReclaimRef<Node<T, R>>> From<Stack<T, R>> for ArcStack<T, R> {
    #[inline]
    fn from(stack: Stack<T, R>) -> Self {
        let inner = Arc::new(stack);
        let thread_state =
            ManuallyDrop::new(unsafe { inner.reclaimer.build_thread_state_unchecked() });

        Self { inner, thread_state }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// StackRef
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A thread-local reference to a [`Stack`].
pub struct StackRef<'s, T, R: ReclaimRef<Node<T, R>>> {
    stack: &'s Stack<T, R>,
    thread_state: R::ThreadState,
}

/********** impl inherent *************************************************************************/

impl<'s, T, R: ReclaimRef<Node<T, R>>> StackRef<'s, T, R> {
    /// Creates a new [`StackRef`] from the given `stack` reference.
    #[inline]
    pub fn new(stack: &'s Stack<T, R>) -> Self {
        Self { stack, thread_state: unsafe { stack.reclaimer.build_thread_state_unchecked() } }
    }
}

impl<'s, T, R: ReclaimRef<Node<T, R>>> StackRef<'s, T, R> {
    /// Returns `true` if the stack is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    /// Pushes `elem` to the top of the stack.
    #[inline]
    pub fn push(&self, elem: T) {
        unsafe { self.stack.push(elem, &self.thread_state) };
    }

    /// Pops the element from the top of the stack or returns [`None`] if the
    /// stack is empty.
    #[inline]
    pub fn pop(&self) -> Option<T> {
        unsafe { self.stack.pop_unchecked(&self.thread_state) }
    }
}

/********** impl Debug ****************************************************************************/

impl<T, R: ReclaimRef<Node<T, R>>> fmt::Debug for StackRef<'_, T, R> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StackRef {{ ... }}")
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Stack
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct Stack<T, R: ReclaimRef<Node<T, R>>> {
    head: Atomic<Node<T, R>, R::Reclaim>,
    reclaimer: R,
}

/********** impl inherent *************************************************************************/

impl<T, R: ReclaimRef<Node<T, R>> + Default> Stack<T, R> {
    #[inline]
    pub fn new() -> Self {
        Self::with_reclaimer(Default::default())
    }
}

impl<T, R: ReclaimRef<Node<T, R>>> Stack<T, R> {
    const RELEASE_CAS: (Ordering, Ordering) = (Release, Relaxed);

    #[inline]
    pub fn with_reclaimer(reclaimer: R) -> Self {
        Self { head: Atomic::null(), reclaimer }
    }

    #[inline]
    pub fn as_ref(&self) -> StackRef<T, R> {
        StackRef::new(self)
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.head.load_unprotected(Ordering::Relaxed).is_null()
    }

    #[inline]
    pub unsafe fn push(&self, elem: T, thread_state: &R::ThreadState) {
        let mut node = thread_state.alloc_owned(Node::new(elem));
        loop {
            let head = self.head.load_unprotected(Acquire);
            // safety: The store only becomes visible if the subsequent CAS succeeds, in which case
            // the head could not have changed and has therefore not been retired by any other thread
            node.next.store(head.assume_storable(), Relaxed);
            match self.head.compare_exchange_weak(head, node, Self::RELEASE_CAS) {
                Ok(_) => return,
                Err(err) => node = err.input,
            }
        }
    }

    #[inline]
    pub unsafe fn pop_unchecked(&self, thread_state: &R::ThreadState) -> Option<T> {
        let mut guard = thread_state.build_guard();
        while let Maybe::Some(shared) = self.head.load(&mut guard, Acquire).shared() {
            // safety: `next` can be safely used as store argument for the subsequent CAS, since it
            // will only be actually stored if it succeeds, in which case the node could not have
            // been popped and retired/reclaimed in between.
            let next = shared.as_ref().next.load_unprotected(Relaxed).assume_storable();
            if let Ok(unlinked) = self.head.compare_exchange_weak(shared, next, Self::RELEASE_CAS) {
                let elem = unlinked.take(|node| &node.elem);
                thread_state.retire_record(unlinked.into_retired());
                return Some(elem);
            }
        }

        None
    }
}

/********** impl Default **************************************************************************/

impl<T, R: ReclaimRef<Node<T, R>> + Default> Default for Stack<T, R> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/********** impl Drop *****************************************************************************/

impl<T, R: ReclaimRef<Node<T, R>>> Drop for Stack<T, R> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            let mut curr = self.head.take();
            while let Some(mut node) = curr {
                ManuallyDrop::drop(&mut node.elem);
                curr = node.next.take();
            }
        }
    }
}

/********** impl IntoIterator *********************************************************************/

impl<T, R: ReclaimRef<Node<T, R>>> IntoIterator for Stack<T, R> {
    type Item = T;
    type IntoIter = IntoIter<T, R>;

    #[inline]
    fn into_iter(mut self) -> Self::IntoIter {
        IntoIter { curr: unsafe { self.head.take() } }
    }
}

/********** impl FromIterator *********************************************************************/

impl<T, R: ReclaimRef<Node<T, R>> + Default> FromIterator<T> for Stack<T, R> {
    #[inline]
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let reclaimer = R::default();

        let head = Atomic::null();
        let mut prev = MarkedPtr::null();

        for elem in iter.into_iter() {
            let node = reclaimer.alloc_owned(Node::new(elem));
            unsafe { node.next.as_raw() }.store(prev, Ordering::Relaxed);
            prev = Owned::as_marked_ptr(&node);
            head.store(node, Ordering::Relaxed);
        }

        Self { head, reclaimer }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// IntoIter
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct IntoIter<T, R: ReclaimRef<Node<T, R>>> {
    curr: Option<Owned<Node<T, R>, R::Reclaim>>,
}

/********** impl Iterator *************************************************************************/

impl<T, R: ReclaimRef<Node<T, R>>> Iterator for IntoIter<T, R> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.curr.take().map(|mut node| unsafe {
            let elem = ptr::read(&*node.elem);
            self.curr = node.next.take();
            elem
        })
    }
}

/********** impl Drop *****************************************************************************/

impl<T, R: ReclaimRef<Node<T, R>>> Drop for IntoIter<T, R> {
    #[inline]
    fn drop(&mut self) {
        while let Some(_) = self.next() {}
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Node
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A node type for storing a [`Stack`]'s individual elements.
pub struct Node<T, R: ReclaimRef<Self>> {
    /// The node's element, which is only ever dropped as part of a node when a
    /// non-empty [`Stack`] itself is dropped.
    elem: ManuallyDrop<T>,
    /// The node's next pointer.
    next: Atomic<Self, R::Reclaim>,
}

/********** impl inherent *************************************************************************/

impl<T, R: ReclaimRef<Self>> Node<T, R> {
    #[inline]
    fn new(elem: T) -> Self {
        Self { elem: ManuallyDrop::new(elem), next: Atomic::default() }
    }
}

/********** impl Drop *****************************************************************************/

#[cfg(feature = "examples-debug")]
impl<T, R: ReclaimRef<Self>> Drop for Node<T, R> {
    #[inline]
    fn drop(&mut self) {
        NODE_DROP_COUNTER.fetch_add(1, Ordering::Relaxed);
    }
}

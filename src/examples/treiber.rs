use core::iter::{FromIterator, IntoIterator};
use core::mem::{self, ManuallyDrop};
use core::ptr;
#[cfg(feature = "examples-debug")]
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering::{self, Acquire, Relaxed, Release};

cfg_if::cfg_if! {
    if #[cfg(feature = "std")] {
        use std::sync::Arc;
    } else {
        use alloc::sync::Arc;
    }
}

use conquer_pointer::MarkedPtr;

use crate::typenum::U0;
use crate::{GlobalReclaim, LocalState, Maybe, Reclaim};

type Atomic<T, R> = crate::Atomic<T, R, U0>;
type Owned<T, R> = crate::Owned<T, R, U0>;

/********** global node drop counter (debug) ******************************************************/

#[cfg(feature = "examples-debug")]
pub static NODE_DROP_COUNTER: AtomicUsize = AtomicUsize::new(0);

////////////////////////////////////////////////////////////////////////////////////////////////////
// ArcStack
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct ArcStack<T, R: Reclaim> {
    inner: Arc<Stack<T, R>>,
    reclaim_local_state: ManuallyDrop<R::LocalState>,
}

/*********** impl Send ****************************************************************************/

unsafe impl<T, R: Reclaim> Send for ArcStack<T, R> {}

/*********** impl Clone ***************************************************************************/

impl<T, R: Reclaim> Clone for ArcStack<T, R> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            reclaim_local_state: ManuallyDrop::new(unsafe {
                self.inner.reclaimer.build_local_state()
            }),
        }
    }
}

/*********** impl inherent ************************************************************************/

impl<T, R: Reclaim> ArcStack<T, R> {
    #[inline]
    pub fn new() -> Self {
        let inner = Arc::new(Stack::<_, R>::new());
        let reclaim_local_state = unsafe { inner.reclaimer.build_local_state() };
        Self { inner, reclaim_local_state: ManuallyDrop::new(reclaim_local_state) }
    }

    #[inline]
    pub fn push(&self, elem: T) {
        self.inner.push(elem)
    }

    #[inline]
    pub fn pop(&self) -> Option<T> {
        unsafe { self.inner.pop_unchecked(&self.reclaim_local_state) }
    }

    #[inline]
    pub fn try_unwrap(self) -> Result<Stack<T, R>, Self> {
        // circumvents the restrictions on moving out of types implementing Drop.
        let (inner, mut reclaim_local_state) = unsafe {
            let inner = ptr::read(&self.inner);
            let reclaimer_thread_state = ptr::read(&self.reclaim_local_state);
            mem::forget(self);
            (inner, reclaimer_thread_state)
        };

        Arc::try_unwrap(inner)
            .map(|stack| {
                unsafe { ManuallyDrop::drop(&mut reclaim_local_state) };
                stack
            })
            .map_err(|inner| Self { inner, reclaim_local_state })
    }
}

/********** impl Default **************************************************************************/

impl<T, R: Reclaim> Default for ArcStack<T, R> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/********** impl Drop *****************************************************************************/

impl<T, R: Reclaim> Drop for ArcStack<T, R> {
    #[inline]
    fn drop(&mut self) {
        // safety: Drop Local state before the `Arc`, because it may hold a pointer into it.
        unsafe { ManuallyDrop::drop(&mut self.reclaim_local_state) };
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Stack
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct Stack<T, R: Reclaim> {
    head: Atomic<Node<T, R>, R>,
    reclaimer: R,
}

/********** impl inherent *************************************************************************/

impl<T, R: Reclaim> Stack<T, R> {
    const RELEASE_CAS: (Ordering, Ordering) = (Release, Relaxed);

    #[inline]
    pub fn new() -> Self {
        Self { head: Atomic::null(), reclaimer: Default::default() }
    }

    #[inline]
    pub fn push(&self, elem: T) {
        let mut node = Owned::new(Node::new(elem));
        loop {
            let head = self.head.load_unprotected(Acquire);
            // safety: The store only becomes visible if the subsequent CAS succeeds, in which case
            // the head could not have changed and has therefore not been retired by any other thread
            node.next.store(unsafe { head.assume_storable() }, Relaxed);
            match self.head.compare_exchange_weak(head, node, Self::RELEASE_CAS) {
                Ok(_) => return,
                Err(err) => node = err.input,
            }
        }
    }

    #[inline]
    pub unsafe fn pop_unchecked(&self, local_state: &R::LocalState) -> Option<T> {
        let mut guard = local_state.build_guard();
        while let Maybe::Some(shared) = self.head.load(&mut guard, Acquire).shared() {
            // safety: `next` can be safely used as store argument for the subsequent CAS, since it
            // will only be actually stored if it succeeds, in which case the node could not have
            // been popped and retired/reclaimed in between.
            let next = shared.as_ref().next.load_unprotected(Relaxed).assume_storable();
            if let Ok(unlinked) = self.head.compare_exchange_weak(shared, next, Self::RELEASE_CAS) {
                let elem = unlinked.take(|node| &node.elem);
                local_state.retire_record(unlinked.into_retired_unchecked());
                return Some(elem);
            }
        }

        None
    }
}

// for global reclaimer, TLS refs can be trivially instantiated from thread-local static variables,
// so a specialization impl helps avoiding the requirement to first create a `StackRef` instance.
impl<T, R: GlobalReclaim> Stack<T, R> {
    #[inline]
    pub fn pop(&self) -> Option<T> {
        unsafe { self.pop_unchecked(&<R as GlobalReclaim>::build_local_state()) }
    }
}

/********** impl Default **************************************************************************/

impl<T, R: Reclaim> Default for Stack<T, R> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/********** impl Drop *****************************************************************************/

impl<T, R: Reclaim> Drop for Stack<T, R> {
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

impl<T, R: Reclaim> IntoIterator for Stack<T, R> {
    type Item = T;
    type IntoIter = IntoIter<T, R>;

    #[inline]
    fn into_iter(mut self) -> Self::IntoIter {
        IntoIter { curr: unsafe { self.head.take() } }
    }
}

/********** impl FromIterator *********************************************************************/

impl<T, R: Reclaim> FromIterator<T> for Stack<T, R> {
    #[inline]
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let head = Atomic::null();
        let mut prev = MarkedPtr::null();

        for elem in iter.into_iter() {
            let node = Owned::new(Node::new(elem));
            unsafe { node.next.as_raw() }.store(prev, Ordering::Relaxed);
            prev = Owned::as_marked_ptr(&node);
            head.store(node, Ordering::Relaxed);
        }

        Self { head, reclaimer: Default::default() }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// StackRef
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct StackRef<'s, T, R: Reclaim> {
    stack: &'s Stack<T, R>,
    reclaimer_local_state: R::LocalState,
}

/********** impl inherent *************************************************************************/

impl<'s, T, R: Reclaim> StackRef<'s, T, R> {
    #[inline]
    pub fn new(stack: &'s Stack<T, R>) -> Self {
        Self { stack, reclaimer_local_state: unsafe { stack.reclaimer.build_local_state() } }
    }

    #[inline]
    pub fn pop(&self) -> Option<T> {
        unsafe { self.stack.pop_unchecked(&self.reclaimer_local_state) }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// IntoIter
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct IntoIter<T, R: Reclaim> {
    curr: Option<Owned<Node<T, R>, R>>,
}

/********** impl Iterator *************************************************************************/

impl<T, R: Reclaim> Iterator for IntoIter<T, R> {
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

impl<T, R: Reclaim> Drop for IntoIter<T, R> {
    fn drop(&mut self) {
        while let Some(_) = self.next() {}
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Node
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A node type for storing a [`Stack`]'s individual elements.
struct Node<T, R: Reclaim> {
    /// The node's element, which is only ever dropped as part of a node when a
    /// non-empty [`Stack`] itself is dropped.
    elem: ManuallyDrop<T>,
    /// The node's next pointer.
    next: Atomic<Self, R>,
}

/********** impl inherent *************************************************************************/

impl<T, R: Reclaim> Node<T, R> {
    #[inline]
    fn new(elem: T) -> Self {
        Self { elem: ManuallyDrop::new(elem), next: Default::default() }
    }
}

/********** impl Drop *****************************************************************************/

#[cfg(feature = "examples-debug")]
impl<T, R: Reclaim> Drop for Node<T, R> {
    #[inline]
    fn drop(&mut self) {
        NODE_DROP_COUNTER.fetch_add(1, Ordering::Relaxed);
    }
}

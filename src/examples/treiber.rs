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
use crate::{Maybe, ReclaimLocalState, ReclaimRef, Retire};

type Atomic<T, R> = crate::Atomic<T, R, U0>;
type Owned<T, R> = crate::Owned<T, R, U0>;

/********** global node drop counter (debug) ******************************************************/

#[cfg(feature = "examples-debug")]
pub static NODE_DROP_COUNTER: AtomicUsize = AtomicUsize::new(0);

////////////////////////////////////////////////////////////////////////////////////////////////////
// ArcStack
////////////////////////////////////////////////////////////////////////////////////////////////////

/// An [`Arc`] based version of Treiber's lock-free stack.
pub struct ArcStack<T, R: ReclaimRef> {
    inner: Arc<Stack<T, R>>,
    reclaim_local_state: ManuallyDrop<R::LocalState>,
}

/*********** impl Send ****************************************************************************/

unsafe impl<T, R: ReclaimRef> Send for ArcStack<T, R> {}

/*********** impl Clone ***************************************************************************/

impl<T, R: ReclaimRef> Clone for ArcStack<T, R> {
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

impl<T, R: ReclaimRef + Default> ArcStack<T, R> {
    #[inline]
    pub fn new() -> Self {
        Self::with_reclaimer(Default::default())
    }
}

impl<T, R: ReclaimRef> ArcStack<T, R> {
    #[inline]
    pub fn with_reclaimer(reclaimer: R) -> Self {
        let inner = Arc::new(Stack::<_, R>::with_reclaimer(reclaimer));
        let reclaim_local_state = unsafe { inner.reclaimer.build_local_state() };
        Self { inner, reclaim_local_state: ManuallyDrop::new(reclaim_local_state) }
    }

    #[inline]
    pub fn try_unwrap(self) -> Result<Stack<T, R>, Self> {
        // circumvents the restrictions on moving out of types implementing Drop.
        let (inner, mut reclaim_local_state) = unsafe {
            let inner = ptr::read(&self.inner);
            let reclaim_local_state = ptr::read(&self.reclaim_local_state);
            mem::forget(self);
            (inner, reclaim_local_state)
        };

        Arc::try_unwrap(inner)
            .map(|stack| {
                unsafe { ManuallyDrop::drop(&mut reclaim_local_state) };
                stack
            })
            .map_err(|inner| Self { inner, reclaim_local_state })
    }
}

impl<T, R: ReclaimRef> ArcStack<T, R>
where
    R::Reclaim: Retire<Node<T, R>>,
{
    #[inline]
    pub fn push(&self, elem: T) {
        self.inner.push(elem)
    }

    #[inline]
    pub fn pop(&self) -> Option<T> {
        unsafe { self.inner.pop_unchecked(&self.reclaim_local_state) }
    }
}

/********** impl Default **************************************************************************/

impl<T, R: ReclaimRef + Default> Default for ArcStack<T, R> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/********** impl Drop *****************************************************************************/

impl<T, R: ReclaimRef> Drop for ArcStack<T, R> {
    #[inline]
    fn drop(&mut self) {
        // safety: Drop Local state before the `Arc`, because it may hold a pointer into it.
        unsafe { ManuallyDrop::drop(&mut self.reclaim_local_state) };
    }
}

/********** impl From (Stack) *********************************************************************/

impl<T, R: ReclaimRef> From<Stack<T, R>> for ArcStack<T, R> {
    #[inline]
    fn from(stack: Stack<T, R>) -> Self {
        let inner = Arc::new(stack);
        let reclaim_local_state = ManuallyDrop::new(unsafe { inner.reclaimer.build_local_state() });

        Self { inner, reclaim_local_state }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Stack
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct Stack<T, R: ReclaimRef> {
    head: Atomic<Node<T, R>, R::Reclaim>,
    reclaimer: R,
}

/********** impl inherent *************************************************************************/

impl<T, R: ReclaimRef + Default> Stack<T, R> {
    #[inline]
    pub fn new() -> Self {
        Self::with_reclaimer(Default::default())
    }
}

impl<T, R: ReclaimRef> Stack<T, R> {
    const RELEASE_CAS: (Ordering, Ordering) = (Release, Relaxed);

    #[inline]
    pub fn with_reclaimer(reclaimer: R) -> Self {
        Self { head: Atomic::null(), reclaimer }
    }
}

impl<T, R: ReclaimRef> Stack<T, R>
where
    R::Reclaim: Retire<Node<T, R>>,
{
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
                local_state.retire_record(unlinked.into_retired());
                return Some(elem);
            }
        }

        None
    }
}

/********** impl Default **************************************************************************/

impl<T, R: ReclaimRef + Default> Default for Stack<T, R> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/********** impl Drop *****************************************************************************/

impl<T, R: ReclaimRef> Drop for Stack<T, R> {
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

impl<T, R: ReclaimRef> IntoIterator for Stack<T, R> {
    type Item = T;
    type IntoIter = IntoIter<T, R>;

    #[inline]
    fn into_iter(mut self) -> Self::IntoIter {
        IntoIter { curr: unsafe { self.head.take() } }
    }
}

/********** impl FromIterator *********************************************************************/

impl<T, R: ReclaimRef + Default> FromIterator<T> for Stack<T, R> {
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

        Self { head, reclaimer: R::default() }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// StackRef
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct StackRef<'s, T, R: ReclaimRef> {
    stack: &'s Stack<T, R>,
    reclaimer_local_state: R::LocalState,
}

/********** impl inherent *************************************************************************/

impl<'s, T, R: ReclaimRef> StackRef<'s, T, R> {
    #[inline]
    pub fn new(stack: &'s Stack<T, R>) -> Self {
        Self { stack, reclaimer_local_state: unsafe { stack.reclaimer.build_local_state() } }
    }
}

impl<'s, T, R: ReclaimRef> StackRef<'s, T, R>
where
    R::Reclaim: Retire<Node<T, R>>,
{
    #[inline]
    pub fn push(&self, elem: T) {
        self.stack.push(elem)
    }

    #[inline]
    pub fn pop(&self) -> Option<T> {
        unsafe { self.stack.pop_unchecked(&self.reclaimer_local_state) }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// IntoIter
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct IntoIter<T, R: ReclaimRef> {
    curr: Option<Owned<Node<T, R>, R::Reclaim>>,
}

/********** impl Iterator *************************************************************************/

impl<T, R: ReclaimRef> Iterator for IntoIter<T, R> {
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

impl<T, R: ReclaimRef> Drop for IntoIter<T, R> {
    #[inline]
    fn drop(&mut self) {
        while let Some(_) = self.next() {}
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Node
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A node type for storing a [`Stack`]'s individual elements.
pub struct Node<T, R: ReclaimRef> {
    /// The node's element, which is only ever dropped as part of a node when a
    /// non-empty [`Stack`] itself is dropped.
    elem: ManuallyDrop<T>,
    /// The node's next pointer.
    next: Atomic<Self, R::Reclaim>,
}

/********** impl inherent *************************************************************************/

impl<T, R: ReclaimRef> Node<T, R> {
    #[inline]
    fn new(elem: T) -> Self {
        Self { elem: ManuallyDrop::new(elem), next: Atomic::default() }
    }
}

/********** impl Drop *****************************************************************************/

#[cfg(feature = "examples-debug")]
impl<T, R: ReclaimRef> Drop for Node<T, R> {
    #[inline]
    fn drop(&mut self) {
        NODE_DROP_COUNTER.fetch_add(1, Ordering::Relaxed);
    }
}

use std::mem::ManuallyDrop;
use std::ptr;
use std::sync::atomic::Ordering;

use conquer_reclaim::conquer_pointer::MaybeNull::NotNull;
use conquer_reclaim::typenum::U0;
use conquer_reclaim::{GlobalReclaim, LocalRef, Owned, Reclaim};

type Atomic<T, R> = conquer_reclaim::Atomic<T, R, U0>;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Stack
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct Stack<T, R: Reclaim> {
    head: Atomic<Node<T, R>, R>,
    reclaimer: R,
}

/********** impl inherent *************************************************************************/

impl<T, R: Reclaim> Stack<T, R> {
    const RELEASE_CAS: (Ordering, Ordering) = (Ordering::Release, Ordering::Relaxed);
    const RELAXED_CAS: (Ordering, Ordering) = (Ordering::Relaxed, Ordering::Relaxed);

    /// Creates a new empty [`Stack`].
    #[inline]
    pub fn new() -> Self {
        Self { head: Atomic::null(), reclaimer: R::new() }
    }

    #[inline]
    pub fn handle(&self) -> StackHandle<T, R> {
        StackHandle { stack: self, local_ref: R::Ref::from_ref(&self.reclaimer) }
    }

    #[inline]
    pub fn push(&self, elem: T) {
        let mut node = Owned::new(Node::new(elem));
        loop {
            let head = self.head.load_unprotected(Ordering::Relaxed);
            node.next.store(head, Ordering::Relaxed);
            // (stack:1) this release CAS syncs-with the acquire load (stack:2)
            match self.head.compare_exchange_weak(head, node, Self::RELEASE_CAS) {
                Ok(_) => return,
                Err(fail) => node = fail.input,
            }
        }
    }

    #[inline]
    pub unsafe fn pop_unchecked(&self, handle: &R::Ref) -> Option<T> {
        let mut guard = handle.clone().into_guard();
        // (stack:2) this acquire load syncs-with the release CAS (stack:1)
        while let NotNull(head) = self.head.load(&mut guard, Ordering::Acquire) {
            let next = head.deref().next.load_unprotected(Ordering::Relaxed);
            if let Ok(unlinked) = self.head.compare_exchange_weak(head, next, Self::RELAXED_CAS) {
                let res = ptr::read(&*unlinked.deref().inner);
                handle.clone().retire(unlinked.into_retired());

                return Some(res);
            }
        }

        None
    }
}

impl<T, R: GlobalReclaim> Stack<T, R> {
    #[inline]
    pub fn pop(&self) -> Option<T> {
        unsafe { self.pop_unchecked(&R::build_local_ref()) }
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
        let mut curr = self.head.take();
        while let Some(mut node) = curr {
            unsafe { ManuallyDrop::drop(&mut node.inner) };
            curr = node.next.take();
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// StackHandle
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct StackHandle<'s, T, R: Reclaim> {
    stack: &'s Stack<T, R>,
    local_ref: R::Ref,
}

/********** impl inherent *************************************************************************/

impl<T, R: Reclaim> StackHandle<'_, T, R> {
    #[inline]
    pub fn push(&self, elem: T) {
        self.stack.push(elem);
    }

    #[inline]
    pub fn pop(&self) -> Option<T> {
        unsafe { self.stack.pop_unchecked(&self.local_ref) }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Node
////////////////////////////////////////////////////////////////////////////////////////////////////

struct Node<T, R> {
    inner: ManuallyDrop<T>,
    next: Atomic<Node<T, R>, R>,
}

/********** impl inherent *************************************************************************/

impl<T, R: Reclaim> Node<T, R> {
    #[inline]
    fn new(elem: T) -> Self {
        Self { inner: ManuallyDrop::new(elem), next: Atomic::null() }
    }
}

fn main() {}

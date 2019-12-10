use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::ptr;
use std::sync::atomic::Ordering;

use conquer_reclaim::conquer_pointer::MaybeNull::NotNull;
use conquer_reclaim::typenum::U0;
use conquer_reclaim::{GenericReclaimer, GlobalReclaimer, Owned, ReclaimerHandle};

type Atomic<T, R> = conquer_reclaim::Atomic<T, R, U0>;
const REL_RLX: (Ordering, Ordering) = (Ordering::Release, Ordering::Relaxed);

////////////////////////////////////////////////////////////////////////////////////////////////////
// Stack
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct Stack<T, R> {
    head: Atomic<Node<T, R>, R>,
    reclaimer: R,
}

/********** impl inherent *************************************************************************/

impl<T, R: GenericReclaimer> Stack<T, R> {
    #[inline]
    pub fn new() -> Self {
        Self { head: Atomic::null(), reclaimer: R::new() }
    }

    #[inline]
    pub fn handle(&self) -> StackHandle<T, R> {
        StackHandle { stack: self, handle: self.reclaimer.create_local_handle() }
    }

    #[inline]
    pub fn push(&self, elem: T) {
        let mut node = Owned::new(Node::new(elem));

        loop {
            let head = self.head.load_unprotected(Ordering::Relaxed);
            node.next.store(head, Ordering::Relaxed);
            match self.head.compare_exchange_weak(head, node, REL_RLX) {
                Ok(_) => return,
                Err(fail) => node = fail.input,
            }
        }
    }

    #[inline]
    pub unsafe fn pop_unchecked(&self, handle: R::Handle) -> Option<T> {
        let mut guard = handle.clone().guard();

        while let NotNull(head) = self.head.load(&mut guard, Ordering::Acquire) {
            let next = head.deref().next.load_unprotected(Ordering::Relaxed);
            if let Ok(unlinked) = self.head.compare_exchange_weak(head, next, REL_RLX) {
                let res = ptr::read(&*unlinked.deref().inner);
                handle.retire(unlinked.into_retired());

                return Some(res);
            }
        }

        None
    }
}

impl<T, R: GlobalReclaimer> Stack<T, R> {
    #[inline]
    pub fn pop(&self) -> Option<T> {
        unsafe { self.pop_unchecked(R::create_handle()) }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// StackHandle
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct StackHandle<'s, T, R: GenericReclaimer> {
    stack: &'s Stack<T, R>,
    handle: R::Handle,
}

/********** impl inherent *************************************************************************/

impl<T, R: GenericReclaimer> StackHandle<'_, T, R> {
    #[inline]
    pub fn push(&self, elem: T) {
        self.stack.push(elem);
    }

    #[inline]
    pub fn pop(&self) -> Option<T> {
        unsafe { self.stack.pop_unchecked(self.handle.clone()) }
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

impl<T, R: GenericReclaimer> Node<T, R> {
    #[inline]
    fn new(elem: T) -> Self {
        Self { inner: ManuallyDrop::new(elem), next: Atomic::null() }
    }
}

fn main() {}

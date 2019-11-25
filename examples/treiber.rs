use std::mem::ManuallyDrop;
use std::ptr;
use std::sync::atomic::Ordering;

use conquer_reclaim::typenum::U0;
use conquer_reclaim::{Owned, Reclaimer, ReclaimerHandle};

type Atomic<T, R> = conquer_reclaim::Atomic<T, R, U0>;

struct Stack<T, R: Reclaimer> {
    head: Atomic<Node<T, R>, R>,
    reclaimer: R,
}

impl<T, R: Reclaimer> Stack<T, R> {
    #[inline]
    pub fn new() -> Self {
        Self { head: Atomic::null(), reclaimer: Default::default() }
    }

    #[inline]
    pub fn push(&self, elem: T, handle: &mut impl ReclaimerHandle<Reclaimer = R>) {
        let mut node = Owned::new(Node::new(elem));
        let mut guard = handle.guard();

        loop {
            let head = self.head.load(&mut guard, Ordering::Relaxed);
            node.as_ref().next.store(head, Ordering::Relaxed);
            match self.head.compare_exchange(head, node, Ordering::Release, Ordering::Relaxed) {
                Ok(_) => return,
                Err(fail) => {
                    node = fail.input;
                }
            }
        }
    }

    #[inline]
    pub fn pop(&self, handle: &mut impl ReclaimerHandle<Reclaimer = R>) -> Option<T> {
        let mut guard = handle.guard();
        while let Some(head) = self.head.load(&mut guard, Ordering::Acquire) {
            let next = head.as_ref().next.load_unprotected(Ordering::Relaxed);
            if let Ok(unlinked) =
                self.head.compare_exchange(head, next, Ordering::Relaxed, Ordering::Relaxed)
            {
                unsafe {
                    let res = ptr::read(&*unlinked.as_ref().inner);
                    unlinked.retire(handle);
                    return Some(res);
                }
            }
        }

        None
    }
}

struct Node<T, R> {
    inner: ManuallyDrop<T>,
    next: Atomic<Node<T, R>, R>,
}

impl<T, R: Reclaimer> Node<T, R> {
    #[inline]
    fn new(elem: T) -> Self {
        Self { inner: ManuallyDrop::new(elem), next: Atomic::null() }
    }
}

fn main() {}

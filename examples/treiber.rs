use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::ptr;
use std::sync::atomic::Ordering;

use conquer_pointer::MarkedOption::Value;
use conquer_reclaim::typenum::U0;
use conquer_reclaim::{Owned, GlobalReclaimer, ReclaimerHandle};

type Atomic<T, R> = conquer_reclaim::Atomic<T, R, U0>;

struct Stack<T, R: GlobalReclaimer> {
    head: Atomic<Node<T, R>, R>,
    _marker: PhantomData<R>,
}

impl<T, R: GlobalReclaimer> Stack<T, R> {
    #[inline]
    pub fn new() -> Self {
        Self { head: Atomic::null(), _marker: PhantomData }
    }

    #[inline]
    pub fn push(&self, elem: T) {
        let mut node = Owned::new(Node::new(elem));

        loop {
            let head = self.head.load_unprotected(Ordering::Relaxed);
            node.next.store(head, Ordering::Relaxed);
            match self.head.compare_exchange_weak(head, node, Ordering::Release, Ordering::Relaxed) {
                Ok(_) => return,
                Err(fail) => node = fail.input
            }
        }
    }

    #[inline]
    pub fn pop(&self) -> Option<T> {
        /*
        let mut guard = R::guard();
        // Option/MarkedOption opt-in
        while let Some(head) = self.head.load(&mut guard, Ordering::Acquire).as_ref() {
            let next = unsafe { head.deref().next.load_unprotected(Ordering::Relaxed);
            if let Ok(Pointer(unlinked)) = self.head.compare_exchange_weak(head, next), Ordering::Release, Ordering::Relaxed) {
                unsafe {
                    let res = ptr::read(&*unlinked.as_ref().inner);
                    unlinked.retire_global();
                    return Some(res);
                }
            }
        }

        None
        */


        let mut guard = handle.guard();

        // while let (Some(head), shared) = self.head.load(&mut guard, Ordering::Acquire).as_ref() { .. }

        loop {
            let head = self.head.load(&mut guard, Ordering::Acquire);
            match unsafe { head.as_ref() } {
                Some(node) => {
                    let next = node.next.load_unprotected(Ordering::Relaxed);
                    if let Ok(Value(unlinked)) = self.head.compare_exchange_weak(head, next, Ordering::Release, Ordering::Relaxed) {
                        unsafe {
                            let res = unimplemented!();
                            //R::retire(unlinked)
                            return Some(res)
                        }
                    }
                }
                None => return None,
            }
        }

        while let Some(head) = self.head.load(&mut guard, Ordering::Acquire).as_ref()


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

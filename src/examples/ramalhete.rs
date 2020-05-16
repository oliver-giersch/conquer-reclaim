use core::cell::UnsafeCell;
use core::mem::{ManuallyDrop, MaybeUninit};
use core::ptr;
use core::sync::atomic::{AtomicU32, AtomicU8, Ordering};

use conquer_util::align::Aligned128 as CacheLineAligned;

use crate::{Owned, ReclaimLocalState, ReclaimRef, Retire};

type Atomic<T, R> = crate::Atomic<T, R, crate::typenum::U0>;

const NODE_SIZE: usize = 1024;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Queue
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A concurrent unbounded lock-free multi-producer/multi-consumer FIFO queue.
///
/// The implementation is based on an algorithm by Andreia Correia and Pedro
/// Ramalhete.
pub struct Queue<T, R: ReclaimRef> {
    head: CacheLineAligned<Atomic<Node<T, R>, R::Reclaim>>,
    tail: CacheLineAligned<Atomic<Node<T, R>, R::Reclaim>>,
    reclaimer: R,
}

/*********** impl inherent ************************************************************************/

impl<T, R: ReclaimRef + Default> Queue<T, R> {
    #[inline]
    pub fn new() -> Self {
        Self::with_reclaimer(Default::default())
    }
}

impl<T, R: ReclaimRef> Queue<T, R> {
    /// The list consists of linked array nodes and this constant defines the
    /// size of each array.
    pub const NODE_SIZE: usize = NODE_SIZE;
    const REL_RLX: (Ordering, Ordering) = (Ordering::Release, Ordering::Relaxed);

    /// Creates a new empty queue.
    #[inline]
    pub fn with_reclaimer(reclaimer: R) -> Self {
        let node = Owned::leak(Owned::new(Node::new()));
        Self {
            head: CacheLineAligned::new(Atomic::from(node)),
            tail: CacheLineAligned::new(Atomic::from(node)),
            reclaimer,
        }
    }
}

impl<T, R: ReclaimRef> Queue<T, R>
where
    R::Reclaim: Retire<Node<T, R>>,
{
    /// Returns `true` if the queue is empty.
    ///
    /// # Safety
    ///
    /// The `local_state` must have been derived from this queue's specific
    /// [`Reclaim`] instance.
    #[inline]
    pub unsafe fn is_empty_unchecked(&self, local_state: &R::LocalState) -> bool {
        let mut guard = local_state.build_guard();
        let head = self.head().load(&mut guard, Ordering::Acquire).shared_unchecked();
        head.as_ref().is_empty()
    }

    /// Pushes `elem` to the tail of the queue.
    ///
    /// # Safety
    ///
    /// The `local_state` must have been derived from this queue's specific
    /// [`Reclaim`] instance.
    #[inline]
    pub unsafe fn push_unchecked(&self, elem: T, local_state: &R::LocalState) {
        let elem = ManuallyDrop::new(elem);
        let mut guard = local_state.build_guard();
        loop {
            let tail = self.tail().load(&mut guard, Ordering::Acquire).shared_unchecked();

            let idx = tail.as_ref().push_idx.fetch_add(1, Ordering::Relaxed) as usize;
            if idx < NODE_SIZE {
                if tail.as_ref().slots[idx].write_tentative(&elem) {
                    return;
                }
            } else {
                if tail.into_marked_ptr() != self.tail().load_raw(Ordering::Relaxed) {
                    continue;
                }

                let next = tail.as_ref().next.load_unprotected(Ordering::Acquire);
                if next.is_null() {
                    let node = Owned::leak(Owned::new(Node::with_tentative(&elem)));
                    if tail.as_ref().next.compare_exchange(next, node, Self::REL_RLX).is_ok() {
                        let _ = self.tail().compare_exchange(tail, node, Self::REL_RLX);
                        return;
                    } else {
                        Owned::from_storable(node);
                    }
                } else {
                    let next = next.assume_storable();
                    let _ = self.tail().compare_exchange(tail, next, Self::REL_RLX);
                }
            }
        }
    }

    /// Pops an element from the head of the queue and returns it or `None`, if
    /// the queue is empty.
    ///
    /// # Safety
    ///
    /// The `local_state` must have been derived from this queue's specific
    /// [`Reclaim`] instance.
    #[inline]
    pub unsafe fn pop_unchecked(&self, local_state: &R::LocalState) -> Option<T> {
        let mut guard = local_state.build_guard();
        loop {
            let head = self.head().load(&mut guard, Ordering::Acquire).shared_unchecked();
            if head.as_ref().is_empty() {
                return None;
            }

            let idx = head.as_ref().pop_idx.fetch_add(1, Ordering::Relaxed) as usize;
            if idx < NODE_SIZE {
                match head.as_ref().slots[idx].try_read() {
                    None => continue,
                    res => return res,
                };
            } else {
                let next = head.as_ref().next.load_unprotected(Ordering::Acquire);
                if next.is_null() {
                    return None;
                }

                if let Ok(unlinked) =
                    self.head().compare_exchange(head, next.assume_storable(), Self::REL_RLX)
                {
                    local_state.retire_record(unlinked.into_retired());
                }
            }
        }
    }

    #[inline]
    fn head(&self) -> &Atomic<Node<T, R>, R::Reclaim> {
        self.head.get()
    }

    #[inline]
    fn tail(&self) -> &Atomic<Node<T, R>, R::Reclaim> {
        self.tail.get()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// QueueRef
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct QueueRef<'q, T, R: ReclaimRef> {
    queue: &'q Queue<T, R>,
    reclaim_local_state: R::LocalState,
}

/********** impl inherent *************************************************************************/

impl<'q, T, R: ReclaimRef> QueueRef<'q, T, R> {
    #[inline]
    pub fn new(queue: &'q Queue<T, R>) -> Self {
        Self { queue, reclaim_local_state: unsafe { queue.reclaimer.build_local_state() } }
    }
}

impl<'q, T, R: ReclaimRef> QueueRef<'q, T, R>
where
    R::Reclaim: Retire<Node<T, R>>,
{
    /// Returns `true` if the queue is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        unsafe { self.queue.is_empty_unchecked(&self.reclaim_local_state) }
    }

    /// Pushes `elem` to the tail of the queue.
    #[inline]
    pub fn push(&self, elem: T) {
        unsafe { self.queue.push_unchecked(elem, &self.reclaim_local_state) }
    }

    /// Pops an element from the head of the queue and returns it or `None`, if
    /// the queue is empty.
    #[inline]
    pub fn pop(&self) -> Option<T> {
        unsafe { self.queue.pop_unchecked(&self.reclaim_local_state) }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Node
////////////////////////////////////////////////////////////////////////////////////////////////////

#[repr(C)]
pub struct Node<T, R: ReclaimRef> {
    pop_idx: AtomicU32,
    slots: [Slot<T>; NODE_SIZE],
    push_idx: AtomicU32,
    next: Atomic<Self, R::Reclaim>,
}

/*********** impl inherent ************************************************************************/

impl<T, R: ReclaimRef> Node<T, R> {
    #[inline]
    fn new() -> Self {
        Self {
            pop_idx: AtomicU32::new(0),
            slots: Self::init_slots(),
            push_idx: AtomicU32::new(0),
            next: Atomic::null(),
        }
    }

    #[inline]
    fn with_tentative(elem: &ManuallyDrop<T>) -> Self {
        let mut slots = Self::init_slots();
        slots[0] = Slot::with_tentative(elem);

        Self {
            pop_idx: AtomicU32::new(0),
            slots,
            push_idx: AtomicU32::new(1),
            next: Atomic::null(),
        }
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.pop_idx.load(Ordering::Relaxed) >= self.push_idx.load(Ordering::Relaxed)
            && self.next.load_raw(Ordering::Relaxed).is_null()
    }

    #[inline]
    fn init_slots() -> [Slot<T>; NODE_SIZE] {
        unsafe {
            let mut slots: MaybeUninit<[Slot<T>; NODE_SIZE]> = MaybeUninit::uninit();
            let ptr: *mut Slot<T> = slots.as_mut_ptr().cast();
            for idx in 0..NODE_SIZE {
                ptr.add(idx).write(Slot::new());
            }

            slots.assume_init()
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Slot
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A slot in the array of a `Node`.
struct Slot<T> {
    /// The slot's element.
    cell: UnsafeCell<MaybeUninit<T>>,
    /// The slot's state.
    state: AtomicU8,
}

/*********** impl inherent ************************************************************************/

impl<T> Slot<T> {
    const RETRY_ATTEMPTS: usize = 8;

    const UNINIT: u8 = 0;
    const WRITER: u8 = 1;
    const READER: u8 = 2;

    #[inline]
    fn new() -> Self {
        Self { cell: UnsafeCell::new(MaybeUninit::zeroed()), state: AtomicU8::new(Self::UNINIT) }
    }

    #[inline]
    fn with_tentative(elem: &ManuallyDrop<T>) -> Self {
        Self {
            cell: UnsafeCell::new(MaybeUninit::new(unsafe { ptr::read(&**elem) })),
            state: AtomicU8::new(Self::WRITER),
        }
    }

    #[inline]
    unsafe fn write_tentative(&self, elem: &ManuallyDrop<T>) -> bool {
        (*self.cell.get()).as_mut_ptr().write_volatile(ptr::read(&**elem));
        self.state.swap(Self::WRITER, Ordering::Release) == Self::UNINIT
    }

    #[inline]
    unsafe fn try_read(&self) -> Option<T> {
        let mut read = None;
        for _ in 0..Self::RETRY_ATTEMPTS {
            if self.state.load(Ordering::Acquire) == Self::WRITER {
                read = Some(self.read_volatile());
                break;
            }
        }

        if self.state.swap(Self::READER, Ordering::Acquire) == Self::WRITER && read.is_none() {
            read = Some(self.read_volatile());
        }

        read
    }

    #[inline]
    unsafe fn read_volatile(&self) -> T {
        (*self.cell.get()).as_ptr().read_volatile()
    }
}

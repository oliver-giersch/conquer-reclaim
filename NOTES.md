# Data Structure API

There are 3 principally possible ways of designing an API for accessing a
concurrent data structure using dynamic memory reclamation:
 
1. The *traditional* way: The data structure can be accessed like any other data
structure, with the exception that all operations only require shared `&self`
receivers, even those that modify the internal state in a synchronized way.
All memory reclamation requirements are abstracted away and hidden through the
use of global (static) singletons. 
 
2. The *rust-friendly* way: Since Rust `std::thread::spawn` requires `'static`
bounds on all types in the closure used for spawning, shared data commonly has
to be wrapped in `Arc`s (thread safe reference-counted smart pointers).
Shared lock-free data structures can be similarly wrapped, using
reference-counted pointers to a shared data structure state, containing both the
data structure itself as well as the "global" (but not static) reclamation
state.
Each handle to this data structure contains a pointer to the shared state, as
well as the necessary thread-local state and must hence be `!Sync`.
    ```rust
    struct Handle {
        shared:  Pin<Arc<DataStructure>>, <--------------------------------|
        reclaim: LocalReclaimState<'self>, // contains a ref into `shared` |
    }
    ```
    This pattern requires a logical self-reference into `shared`, which can be
    realized using a raw pointer in this case without introducing unsoundness.
   
3. The *traditional* way but with explicit memory reclamation:
The approach outlined in 1) has several drawbacks in a library context due to
the reliance on global singletons and OS mechanisms for thread-local storage.
The approach in 2) on the other hand has the (minor) drawback of requiring an
additional indirection through the `Arc` pointer and its associated overhead.
It is also possible to avoid this indirection when using (scoped) threads, that
are able to reference the stack-frame of the spawning parent thread.
In this case, each thread accessing the data structure can create a borrowing
handle to it, that also contains the thread's local state for reclamation:
    ```rust
    struct RefHandle<'d> {
        shared:  &'d DataStructure,
        reclaim: LocalReclaimState<'d>,
    }  
    ```

# Lessons Learned

- de-referencing `Shared` and `Unlinked` is **NEVER** safe since there might be
  data races on the contained fields, if relaxed orderings are used incorrectly,
  but requiring sequential consistency for the sake of safety is exagerated

- ~~non-global fully generic reclamation is possible with GAT, but can not be
  guaranteed to be safe => not worth the effort?~~
  
- non-global fully generic reclamation is possible in stable Rust, but requires
a (partially) unsafe API

- a safe but quite complex API (using HRTB and lifetime-generic traits) is
possible, but can not cover all valid use cases  
  
- separation into `Shared`, `Option<Shared>`, `MarkedOption<Shared>`, etc. is
  clumsy, unwieldy
  
- ...but allows for ergonomic usage of `while let` and similar language
  constructs => compromise solution
  
- global (static) reclamation is necessarily in-efficient to some degree in a library
context
    - unrelated information (from unrelated data structures) is lumped together,
    if multiple data structures use the same global reclamation state
    - requires type erasure and virtual drop methods (results in both space and runtime
    overhead, although minor)

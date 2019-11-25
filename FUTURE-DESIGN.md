# Future API Design

## ReclaimerScheme

Common collection of associated types.

```rust
pub trait ReclaimerScheme {
    type Header: Default + Sync + Sized;
    type Global: Default + Sync + Sized;
}
```

## GlobalReclaimer

```rust
pub trait GlobalReclaimer: Reclaimer {
    fn guard() -> Self::Handle::Guard;
    unsafe fn retire(retired: Retired<Self::Scheme>); 
}
```

## Reclaimer

The handle to global state.

### Notes

The associated `Handle` type is lifetime generic and may store either a
reference to the global state/handle or an owning shared pointer (`Arc`).

```rust
pub trait Reclaimer: Default + Sync + Sized + 'static {
    /// The associated reclamation scheme types.
    type Scheme: ReclaimerScheme;
    /// Handle to local state, may or may not store a reference to
    /// global state.
    type Handle<'global>: ReclaimerHandle<Reclaimer = Self> + 'global;
    
    // Creates a new local state that maintains and a handle for it.
    fn create_handle(&self) -> Self::Handle<'_>;
}
```

## ReclaimerHandle

The handle to the thread-local state.

### Notes

The handle may be a lifetime-bound reference to the local state, in which case
the guard may also be lifetime-bound and store that reference. 

```rust
pub trait ReclaimerHandle: Clone + Sized {
    type Reclaimer: Reclaimer;
    type Guard<'local, 'global: 'local>: Protect<Reclaimer = Self::Reclaimer> + 'local; 
    
    fn guard(self) -> Self::Guard<'_, '_>;
    unsafe fn retire(self, retired: Retired<Self::Reclaimer::Scheme>);
}
```

## Protect

```rust
pub unsafe trait Protect: Sized + Clone {
    type Reclaimer: Reclaim;

    fn release(&mut self);

    fn protect<T, const N: usize>(
        &mut self,
        src: &Atomic<T, Self::Reclaimer, N>,
        order: Ordering
    ) -> MarkedOption<Shared<T, Self::Reclaimer, N>>;
    
    fn protect_if_equal<T, const N: usize>(
        &mut self,
        src: &Atomic<T, Self::Reclaimer, N>,
        expected: MarkedPtr<T, N>,
        order: Ordering
    ) -> Result<MarkedOption<Shared<T, Self::Reclaimer>>, NotEqual>;
}
```

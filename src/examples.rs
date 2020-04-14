//! Reclamation-agnostic implementations of some common lock-free data
//! structures.

/// Reclaimer-generic implementation of the Michael-Scott queue.
pub mod michael_scott;
pub mod ramalhete;
pub mod treiber;

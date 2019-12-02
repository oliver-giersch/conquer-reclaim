# Lessons Learned

- de-referencing `Shared` and `Unlinked` is **NEVER** safe since there might be
  data races on the contained fields, if relaxed orderings are used incorrectly

- non-global fully generic reclamation is possible with GAT, but can not be
  guaranteed to be safe => not worth the effort
  
- separation into `Shared`, `Option<Shared>`, `MarkedOption<Shared>`, etc. is
  clumsy, unwieldy
  
- ...but allows for ergonomic usage of `while let` and similar language
  constructs

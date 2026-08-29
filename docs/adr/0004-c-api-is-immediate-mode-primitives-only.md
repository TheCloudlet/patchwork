# The C API is immediate mode and exposes primitives only

C callers drive the loop themselves: acquire the root Surface for a frame, carve
it up with `pw_surface_sub`, draw primitives, present. There is no C access to
the Pane tree, and no way for C to define a new kind of drawable. This is forced
as much as chosen — `Box<dyn Draw>` is a Rust vtable that C cannot construct or
implement, and `Pane` is built from `Vec`s that C cannot own — so the boundary
lands where the types are already C-shaped: `Surface` is a pointer plus a
rectangle, and the primitives are plain functions over it.

## Considered Options

- **Bind the whole toolkit.** Rejected: pane construction across FFI means C owns
  tree nodes, which means answering who frees a child when its parent is freed.
  That complexity buys nothing for either goal this project has.
- **Callbacks, so C can implement `Draw`.** Rejected for now, not on principle —
  it is the natural next step. It requires `catch_unwind` at the boundary (a Rust
  panic must not unwind into C) and untyped `void*` userdata, neither of which
  belongs in a first cut. The primitives-only API is a subset of it, so adding
  callbacks later breaks nothing.

## Consequences

- The C and Rust sides now share one mental model. Both are immediate mode; the
  Pane tree is a Rust-side convenience for describing a frame, not a privileged
  layer that C is missing out on.
- A `pw_surface_t*` is valid only for the frame it was obtained in — the same
  contract Xlib gives for a GC. Nothing crossing the boundary is owned or freed
  by the caller.
- Layout in C is manual: compute rectangles, then `pw_surface_sub`. Acceptable,
  and a useful check that `Surface` is self-sufficient without the Pane tree.
- `Rect`, `Style`, and `Color` need `#[repr(C)]`, which constrains how they may
  change later.

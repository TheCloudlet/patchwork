# The Pane tree is rebuilt every frame, and that is not an oversight

`Pane` holds `Vec<Box<dyn Draw>>` and `Vec<Pane>`, and callers construct the
whole tree from scratch on every frame, only to drop it at the end. This looks
like an unoptimised retained scene graph — paying for heap allocation and
dynamic dispatch — but it is immediate mode written declaratively: the tree is a
*description of one frame*, never a scene that is kept and mutated. We keep it
because the allocations are nanoseconds against a terminal write measured in
microseconds, and because an intermediate representation is worth having.

## Considered Options

- **Static dispatch via generics or an enum of shapes.** Rejected: it trades
  real type-system complexity — generic parameters that propagate, or an enum
  that stops callers adding drawables — for an optimisation we have not measured
  a need for.
- **Delete `Pane`; call drawing functions directly.** Rejected, though it is the
  cheapest at runtime. Building the tree first is an *initial encoding*: it
  exists as data before it is interpreted, which is what would later make hit
  testing a tree traversal rather than a parallel bookkeeping problem. Removing
  `Pane` closes that door, and mouse support is on the roadmap.

## Consequences

- Nothing about the tree may become load-bearing across frames. No caching, no
  dirty flags, no identity. Frame-to-frame optimisation happens in the renderer's
  cell diff, which is where it already works.
- If allocation ever does show up in a profile, static dispatch remains available
  and the change is contained, since `Surface` already isolates drawables from
  the buffer.

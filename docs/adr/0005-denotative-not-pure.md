# Denotative interfaces, mutating implementations

A stated goal for this project is that it feel functional — Haskell-flavoured,
inputs to outputs. We interpret that as **denotative** rather than **pure**: a
frame is a function of state with no hidden state anywhere, but the
implementation underneath is free to mutate. `Draw::draw(&self, &mut Surface)`
is pure mutation and returns nothing, and that is fine; what matters is that the
same state always produces the same screen, and that nothing is carried between
frames except state the caller is holding explicitly.

## Considered Options

- **Actually pure drawing** — `draw` returns a cell list or a new buffer, which
  the framework composes. Rejected: in Haskell this is affordable because of GC
  and laziness; in Rust it means allocating and merging vectors every frame, and
  the mutation still happens at the end inside the framebuffer. It sacrifices the
  substance for the form.
- **Not pursuing this at all.** Rejected because several decisions already follow
  from it and would otherwise look arbitrary — see below.

## Consequences

Decisions elsewhere in this project are downstream of this one:

- **Frames start blank** (see `CONTEXT.md` → Frame). Reusing the previous frame's
  content would be hidden state, and with double buffering the leftovers are two
  frames old, not one.
- **Scrolling is a coordinate** (ADR-0002). A scroll register would be persistent
  hidden state in the addressing path.
- **The Pane tree may not survive a frame** (ADR-0003). It is a description, not
  an object graph that gets updated in place.

Where purity already exists and is worth protecting: `diff(old, new) ->
BufferDiff` is a genuine pure function over two immutable inputs, and it sits on
the performance-critical path — evidence that the two goals do not conflict here.
`Buffer` is a flat `Vec<Cell>` of `Copy` PODs rather than an object graph, which
is both the data-oriented choice and the denotative one.

The parts that are irreducibly effectful — `Terminal`, `RawMode` — stay that way.
RAII cleanup of a TTY is the correct design and should not be contorted.

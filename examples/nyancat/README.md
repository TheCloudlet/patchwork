# nyancat — a Patchwork C API demo

A terminal Nyan Cat animation, driving Patchwork's immediate-mode C API
(`src/ffi.rs`) end to end from a real C++ program: `pw_renderer_new`,
`pw_frame_begin`, the `pw_draw_*` primitives, and `pw_present`, all called
directly through the RAII wrappers `main.cpp` defines.

## Upstream

The animation data (`frames.h`'s `kCatFrames`, `kCatColorIndex`, and
`kRainbowBand`) is ported byte-for-byte from
[klange/nyancat](https://github.com/klange/nyancat), specifically its
`src/animation.c` (the 12-frame, 64×64 character grid) and `src/nyancat.c`
(the character-to-256-color mapping, the tail's square-wave color formula,
and the auto-fit crop/center formula — all reproduced verbatim in
`main.cpp`'s `draw_frame`).

klange/nyancat is provided under the
[NCSA license](http://en.wikipedia.org/wiki/University_of_Illinois/NCSA_Open_Source_License).
Per its own README:

> The original source of the Nyancat animation is
> [prguitarman](http://www.prguitarman.com/index.php?id=348).

This demo's own code (the render loop, RAII wrappers, SIGWINCH handling,
build files) is original; only the animation's character grid, color
mapping, and crop/tail-wave formulas are copied from klange/nyancat.

## Building and running

```sh
make          # cargo build the staticlib, then compile+link this demo
make run      # build, then run the demo
make clean    # remove this demo's build output
```

The animation auto-fits to whatever terminal size you run it at — cropped
and centered, same as upstream — so there's no minimum size requirement.
Resizing the terminal window live is handled too: the demo installs its own
`SIGWINCH` handler and rebuilds the renderer at the new size (see the
comment above `handle_sigwinch` in `main.cpp` for why that lives on this
side of the FFI boundary, not inside Patchwork itself).

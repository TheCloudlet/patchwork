// C/C++ header for Patchwork's immediate-mode FFI layer (src/ffi.rs).
// Hand-written to mirror the #[repr(C)] / #[repr(C, u8)] layouts verified
// against rustc's actual output — not auto-generated, since the crate has
// no cbindgen setup and this is a one-off verification demo.
#ifndef PATCHWORK_H
#define PATCHWORK_H

#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// Mirrors src/shape.rs Rect: #[repr(C)] { x: u16, y: u16, w: u16, h: u16 }.
typedef struct {
    uint16_t x, y, w, h;
} pw_rect_t;

// Mirrors src/buffer.rs Color: #[repr(C, u8)] enum { Default, Indexed(u8),
// Rgb(u8,u8,u8) }. Rust lays this out as a 1-byte discriminant followed by
// the largest variant's payload (3 bytes), padded to a 4-byte, 1-aligned
// struct — verified with std::mem::size_of/align_of and a raw byte dump.
typedef struct {
    uint8_t tag; // 0 = Default, 1 = Indexed, 2 = Rgb
    uint8_t a, b, c; // Indexed uses only `a`; Rgb uses a=r, b=g, c=b
} pw_color_t;

static inline pw_color_t pw_color_default(void) {
    pw_color_t c = {0, 0, 0, 0};
    return c;
}
static inline pw_color_t pw_color_indexed(uint8_t index) {
    pw_color_t c = {1, index, 0, 0};
    return c;
}
static inline pw_color_t pw_color_rgb(uint8_t r, uint8_t g, uint8_t b) {
    pw_color_t c = {2, r, g, b};
    return c;
}

// Mirrors src/buffer.rs Style: #[repr(C)] { fg: Color, bg: Color, bold: bool,
// underline: bool }.
typedef struct {
    pw_color_t fg;
    pw_color_t bg;
    bool bold;
    bool underline;
} pw_style_t;

static inline pw_style_t pw_style_default(void) {
    pw_style_t s;
    s.fg = pw_color_default();
    s.bg = pw_color_default();
    s.bold = false;
    s.underline = false;
    return s;
}

// Opaque handles — never dereferenced from C/C++, only passed back to pw_*.
typedef struct PwRenderer pw_renderer_t;
typedef struct Frame pw_frame_t;

pw_renderer_t *pw_renderer_new(uint16_t rows, uint16_t cols);
void pw_renderer_free(pw_renderer_t *renderer);

pw_frame_t *pw_frame_begin(pw_renderer_t *renderer);
uint16_t pw_frame_width(pw_frame_t *frame);
uint16_t pw_frame_height(pw_frame_t *frame);

void pw_surface_sub(pw_frame_t *frame, pw_rect_t area);
void pw_surface_end(pw_frame_t *frame);
void pw_clear(pw_frame_t *frame);

void pw_draw_dot(pw_frame_t *frame, uint16_t x, uint16_t y, pw_style_t style);
void pw_draw_line(pw_frame_t *frame, uint16_t x1, uint16_t y1, uint16_t x2,
                   uint16_t y2, pw_style_t style);
void pw_draw_rect(pw_frame_t *frame, pw_rect_t area, pw_style_t style,
                   bool fill);
void pw_draw_text(pw_frame_t *frame, uint16_t x, uint16_t y, uint16_t w,
                   uint16_t h, const char *text, pw_style_t style);

// Ends the frame and frees it. `frame` must not be used afterward.
void pw_present(pw_frame_t *frame);

#ifdef __cplusplus
}
#endif

#endif // PATCHWORK_H

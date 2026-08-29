//! Integration tests for the immediate-mode C API: the exported
//! `pw_*` functions, called directly as Rust would call any `extern "C"`
//! function, asserting on the resulting grid. No C toolchain involved —
//! this covers the logic and the `#[repr(C)]` layouts the same way a real C
//! caller would exercise them.

use patchwork::buffer::{Color, Style};
use patchwork::ffi::*;
use patchwork::shape::Rect;
use std::ffi::CString;

fn render(buf: &patchwork::buffer::Buffer) -> String {
    (0..buf.rows())
        .map(|y| (0..buf.cols()).map(|x| buf.get(x, y).unwrap().ch).collect())
        .collect::<Vec<String>>()
        .join("\n")
}

#[test]
fn begin_draw_present_round_trip() {
    unsafe {
        let renderer = pw_renderer_new(1, 1);
        let frame = pw_frame_begin(renderer);
        pw_draw_dot(frame, 0, 0, Style::DEFAULT);
        pw_present(frame);

        assert_eq!(render((*renderer).current()), "•");
        pw_renderer_free(renderer);
    }
}

#[test]
fn frame_width_and_height_report_the_root_size() {
    unsafe {
        let renderer = pw_renderer_new(3, 5);
        let frame = pw_frame_begin(renderer);
        assert_eq!(pw_frame_width(frame), 5);
        assert_eq!(pw_frame_height(frame), 3);
        pw_present(frame);
        pw_renderer_free(renderer);
    }
}

#[test]
fn sub_narrows_and_end_restores_the_parent_region() {
    unsafe {
        let renderer = pw_renderer_new(4, 4);
        let frame = pw_frame_begin(renderer);

        pw_surface_sub(
            frame,
            Rect {
                x: 1,
                y: 1,
                w: 2,
                h: 2,
            },
        );
        assert_eq!(pw_frame_width(frame), 2);
        assert_eq!(pw_frame_height(frame), 2);

        pw_surface_end(frame);
        assert_eq!(pw_frame_width(frame), 4);
        assert_eq!(pw_frame_height(frame), 4);

        pw_present(frame);
        pw_renderer_free(renderer);
    }
}

#[test]
fn a_narrowed_region_cannot_escape_its_parent() {
    // The regression this whole design exists to prevent, exercised from the
    // C surface: a sub-region asking for far more than its parent has must
    // not reach past it. Asserted on style, since a fill's glyph is a blank
    // and can't be told apart from "untouched" by character alone.
    let marker = Style {
        bg: Color::Indexed(1),
        ..Style::DEFAULT
    };
    unsafe {
        let renderer = pw_renderer_new(4, 8);
        let frame = pw_frame_begin(renderer);

        pw_surface_sub(
            frame,
            Rect {
                x: 0,
                y: 0,
                w: 4,
                h: 4,
            },
        ); // left half only
        pw_surface_sub(
            frame,
            Rect {
                x: 0,
                y: 0,
                w: 99,
                h: 99,
            },
        ); // wants everything
        pw_draw_rect(
            frame,
            Rect {
                x: 0,
                y: 0,
                w: 99,
                h: 99,
            },
            marker,
            true,
        );
        pw_present(frame);

        // The left half (4 cols) is filled; the right half (4 cols) is
        // untouched — no bleed past the parent's boundary.
        let buf = (*renderer).current();
        for y in 0..4 {
            assert!((0..4).all(|x| buf.get(x, y).unwrap().style == marker));
            assert!((4..8).all(|x| buf.get(x, y).unwrap().style == Style::DEFAULT));
        }
        pw_renderer_free(renderer);
    }
}

#[test]
fn draw_line_and_rect_outline() {
    unsafe {
        let renderer = pw_renderer_new(3, 3);
        let frame = pw_frame_begin(renderer);
        pw_draw_rect(
            frame,
            Rect {
                x: 0,
                y: 0,
                w: 3,
                h: 3,
            },
            Style::DEFAULT,
            false,
        );
        pw_present(frame);
        assert_eq!(render((*renderer).current()), "┌─┐\n│ │\n└─┘");
        pw_renderer_free(renderer);
    }
}

#[test]
fn draw_line_connects_two_points() {
    unsafe {
        let renderer = pw_renderer_new(1, 4);
        let frame = pw_frame_begin(renderer);
        pw_draw_line(frame, 0, 0, 3, 0, Style::DEFAULT);
        pw_present(frame);
        assert_eq!(render((*renderer).current()), "····");
        pw_renderer_free(renderer);
    }
}

#[test]
fn draw_text_wraps_and_truncates_like_the_rust_side() {
    unsafe {
        let renderer = pw_renderer_new(2, 3);
        let frame = pw_frame_begin(renderer);
        let text = CString::new("helloo").unwrap();
        pw_draw_text(frame, 0, 0, 3, 2, text.as_ptr(), Style::DEFAULT);
        pw_present(frame);
        assert_eq!(render((*renderer).current()), "hel\nloo");
        pw_renderer_free(renderer);
    }
}

#[test]
fn clear_resets_only_the_current_region() {
    unsafe {
        let renderer = pw_renderer_new(1, 4);
        let frame = pw_frame_begin(renderer);
        pw_draw_line(frame, 0, 0, 3, 0, Style::DEFAULT);

        pw_surface_sub(
            frame,
            Rect {
                x: 1,
                y: 0,
                w: 2,
                h: 1,
            },
        );
        pw_clear(frame);
        pw_surface_end(frame);

        pw_present(frame);
        assert_eq!(render((*renderer).current()), "·  ·");
        pw_renderer_free(renderer);
    }
}

#[test]
fn style_and_color_layouts_round_trip_through_the_c_repr() {
    // Exercises the #[repr(C)]/#[repr(C, u8)] layouts on Rect/Style/Color by
    // passing every Color variant across the boundary and reading the result
    // back through the safe Rust API.
    unsafe {
        let renderer = pw_renderer_new(1, 3);
        let frame = pw_frame_begin(renderer);
        pw_draw_dot(
            frame,
            0,
            0,
            Style {
                fg: Color::Default,
                ..Style::DEFAULT
            },
        );
        pw_draw_dot(
            frame,
            1,
            0,
            Style {
                fg: Color::Indexed(7),
                ..Style::DEFAULT
            },
        );
        pw_draw_dot(
            frame,
            2,
            0,
            Style {
                fg: Color::Rgb(10, 20, 30),
                ..Style::DEFAULT
            },
        );
        pw_present(frame);

        let buf = (*renderer).current();
        assert_eq!(buf.get(0, 0).unwrap().style.fg, Color::Default);
        assert_eq!(buf.get(1, 0).unwrap().style.fg, Color::Indexed(7));
        assert_eq!(buf.get(2, 0).unwrap().style.fg, Color::Rgb(10, 20, 30));
        pw_renderer_free(renderer);
    }
}

#[test]
fn a_frame_begins_blank_regardless_of_the_previous_one() {
    unsafe {
        let renderer = pw_renderer_new(1, 1);

        let frame1 = pw_frame_begin(renderer);
        pw_draw_dot(frame1, 0, 0, Style::DEFAULT);
        pw_present(frame1);
        assert_eq!(render((*renderer).current()), "•");

        // Frame 2 paints nothing; the dot must not survive.
        let frame2 = pw_frame_begin(renderer);
        pw_present(frame2);
        assert_eq!(render((*renderer).current()), " ");

        pw_renderer_free(renderer);
    }
}

// Original, hand-drawn cat frames for the Patchwork FFI verification demo.
// Not derived from any existing Nyan Cat asset — the char-indexed
// color-lookup TECHNIQUE (map a character to a terminal color) is a common
// pattern seen in prior art like klange/nyancat, but every glyph and color
// choice here is drawn from scratch for this demo.
#ifndef NYANCAT_FRAMES_H
#define NYANCAT_FRAMES_H

#include "patchwork.h"

// Each frame is a small sprite, 9 cols x 3 rows. '.' = transparent (don't
// paint). Ears/legs alternate across frames to animate a "flying" wiggle;
// the body itself stays put, matching the real Nyan Cat's silhouette.
static const char *const kCatFrames[][3] = {
    // Frame 0: ears up, legs down
    {
        "^..^....",
        "(oo)====",
        ".||.||..",
    },
    // Frame 1: ears up, legs tucked
    {
        "^..^....",
        "(oo)====",
        "..||||..",
    },
    // Frame 2: ears down (mid-flap), legs down
    {
        "-..-....",
        "(oo)====",
        ".||.||..",
    },
    // Frame 3: ears down, legs tucked
    {
        "-..-....",
        "(oo)====",
        "..||||..",
    },
};
static const int kCatFrameCount = 4;
static const int kCatWidth = 8;
static const int kCatHeight = 3;

// Rainbow trail palette, cycled left-to-right and shifted every animation
// tick to read as a flowing stream behind the cat.
static const uint8_t kRainbow[][3] = {
    {235, 64, 52},  // red
    {235, 151, 52}, // orange
    {235, 220, 52}, // yellow
    {88, 235, 52},  // green
    {52, 137, 235}, // blue
    {129, 52, 235}, // violet
};
static const int kRainbowCount = 6;

#endif // NYANCAT_FRAMES_H

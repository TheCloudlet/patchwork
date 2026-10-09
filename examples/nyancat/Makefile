# Builds this demo (main.cpp) against patchwork's C API (src/ffi.rs). Build
# output goes to build/, which is entirely gitignored — no per-binary-name
# pattern to keep in sync as new binaries get added.
#
# Usage:
#   make         # cargo build the staticlib, then compile+link the demo
#   make run     # build, then run the demo
#   make clean   # remove build/ (leaves Rust's target/ alone)

CXX      ?= g++
CXXFLAGS ?= -std=c++23 -Wall -Wextra -O2
REPO_ROOT := $(abspath ../..)
LIB_DIR   := $(REPO_ROOT)/target/debug
BUILD_DIR := build

BIN := $(BUILD_DIR)/nyancat_demo

.PHONY: all run clean cargo-build

all: $(BIN)

$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)

# Rust's own change detection makes this cheap to always invoke; `cargo
# build` no-ops when nothing changed.
cargo-build:
	cd $(REPO_ROOT) && cargo build

$(BIN): main.cpp frames.h patchwork.h cargo-build | $(BUILD_DIR)
	$(CXX) $(CXXFLAGS) -o $(BIN) main.cpp -L$(LIB_DIR) -lpatchwork -lpthread

run: $(BIN)
	./$(BIN)

clean:
	rm -rf $(BUILD_DIR)

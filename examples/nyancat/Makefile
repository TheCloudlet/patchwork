# Builds this demo (main.cpp) and a separate FFI smoke test (verify.cpp)
# against Patchwork's C API (src/ffi.rs). `verify` exercises only the FFI
# boundary itself and knows nothing about this demo's content, so it stays
# useful even if the demo above it changes completely.
#
# Usage:
#   make         # cargo build the staticlib, then compile+link the demo
#   make run     # build, then run the demo
#   make verify  # build and run the FFI smoke test (no crash / correct exit code)
#   make clean   # remove this demo's build output (leaves Rust's target/ alone)

CXX      ?= g++
CXXFLAGS ?= -std=c++17 -Wall -Wextra -O2
REPO_ROOT := $(abspath ../..)
LIB_DIR   := $(REPO_ROOT)/target/debug
BIN       := nyancat_demo
VERIFY_BIN := nyancat_verify

.PHONY: all run verify clean cargo-build

all: $(BIN)

# Rust's own change detection makes this cheap to always invoke; `cargo
# build` no-ops when nothing changed.
cargo-build:
	cd $(REPO_ROOT) && cargo build

$(BIN): main.cpp frames.h patchwork.h cargo-build
	$(CXX) $(CXXFLAGS) -o $(BIN) main.cpp -L$(LIB_DIR) -lpatchwork -lpthread

$(VERIFY_BIN): verify.cpp patchwork.h cargo-build
	$(CXX) $(CXXFLAGS) -o $(VERIFY_BIN) verify.cpp -L$(LIB_DIR) -lpatchwork -lpthread

run: $(BIN)
	./$(BIN)

verify: $(VERIFY_BIN)
	./$(VERIFY_BIN)

clean:
	rm -f $(BIN) $(VERIFY_BIN)

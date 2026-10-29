RUSTUP_TOOLCHAIN ?= 1.98.1
RUST_ENV := RUSTUP_TOOLCHAIN=$(RUSTUP_TOOLCHAIN)
GUEST_BIN := methods/guest/target/riscv32im-risc0-zkvm-elf/osm_registry.bin
IDL := idl/osm-registry.idl.json
FFI_SO := osm-module/lib/libosm_ffi.so

.PHONY: build ffi idl cli deploy inspect fmt clippy test clean all

all: build ffi idl

# Uses risc0-build's embed_methods pattern and the rzup-installed toolchains,
# then wraps the raw ELF in the v0.3.1 program-loader ProgramBinary format.
build:
	PATH="$(HOME)/.rustup/toolchains/$(RUSTUP_TOOLCHAIN)-x86_64-unknown-linux-gnu/bin:$(HOME)/.risc0/bin:$(PATH)" $(RUST_ENV) cargo build --release --manifest-path methods/Cargo.toml
	$(RUST_ENV) cargo build --release --manifest-path tools/Cargo.toml --bin mk-program-binary
	mkdir -p $$(dirname $(GUEST_BIN))
	GUEST_ELF=$$(find methods/target/riscv-guest -type f -name osm_registry | head -1); test -n "$$GUEST_ELF"; tools/target/release/mk-program-binary $$GUEST_ELF $(GUEST_BIN)
	@echo "Built: $(GUEST_BIN)"

ffi:
	cd ffi && cargo build --release
	mkdir -p osm-module/lib
	cp ffi/target/release/libosm_ffi.so $(FFI_SO)
	@echo "FFI built: $(FFI_SO)"

idl:
	mkdir -p idl
	$(RUST_ENV) cargo run --manifest-path tools/Cargo.toml --bin generate_idl > $(IDL)

cli:
	$(RUST_ENV) cargo build --manifest-path examples/Cargo.toml --bin osm_registry_cli

deploy:
	@echo "Use the v0.3.1 wallet: wallet program-loader deploy --elf $(GUEST_BIN) --header <public-account> --segments <public-account>... --immutable --payer <funded-account>"

inspect:
	@echo "ProgramId is printed by the wrapper during make build."

fmt:
	$(RUST_ENV) cargo fmt --all
	cd ffi && $(RUST_ENV) cargo fmt --all

clippy:
	$(RUST_ENV) cargo clippy --all-targets -- -D warnings
	cd ffi && $(RUST_ENV) cargo clippy --all-targets -- -D warnings

test:
	RISC0_DEV_MODE=1 $(RUST_ENV) cargo test --workspace
	cd ffi && RISC0_DEV_MODE=1 $(RUST_ENV) cargo test

clean:
	$(RUST_ENV) cargo clean
	cd ffi && $(RUST_ENV) cargo clean
	rm -f $(IDL) $(GUEST_BIN) $(FFI_SO)

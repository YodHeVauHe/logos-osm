GUEST_SRC   := methods/guest/src/bin/osm_registry.rs
# cargo-risczero >=3.0.6 exports the ELF under release/, older ones under docker/.
GUEST_ELF_GLOB := methods/guest/target/riscv32im-risc0-zkvm-elf/*/osm_registry
GUEST_BIN   := methods/guest/target/riscv32im-risc0-zkvm-elf/osm_registry.bin
IDL         := idl/osm-registry.idl.json
FFI_SO      := osm-module/lib/libosm_ffi.so

.PHONY: build ffi idl cli deploy inspect fmt clippy test clean all

all: build ffi idl

# Build the RISC0 guest binary deterministically via `cargo risczero build`
# (runs the risc0 docker toolchain internally — no host-specific paths),
# then wrap the ELF into the risc0 ProgramBinary format spel/lgs consume.
build:
	cargo risczero build --manifest-path methods/guest/Cargo.toml
	cargo build --release --manifest-path tools/Cargo.toml --bin mk-program-binary
	GUEST_ELF=$$(ls $(GUEST_ELF_GLOB) | head -1); 	tools/target/release/mk-program-binary $$GUEST_ELF $(GUEST_BIN)
	@echo "Built: $(GUEST_BIN)"
	@command -v spel >/dev/null 2>&1 \
		&& spel program-id $(GUEST_BIN) --format hex \
		|| echo "(spel not on PATH — ProgramId already printed by mk-program-binary above)"

# Build the Rust FFI cdylib used by osm-module (Basecamp core module).
ffi:
	cd ffi && cargo build --release
	mkdir -p osm-module/lib
	cp ffi/target/release/libosm_ffi.so $(FFI_SO)
	@echo "FFI built: $(FFI_SO)"

# Regenerate the IDL from the guest source; the result is committed.
idl:
	mkdir -p idl
	cargo run --manifest-path examples/Cargo.toml --bin generate_idl > $(IDL)

# Build the SPEL instruction CLI used by the SDK's registry client.
cli:
	cargo build --manifest-path examples/Cargo.toml --bin osm_registry_cli

deploy:
	@echo "Extracting program_id from binary..."
	@PROGRAM_ID=$$(spel program-id $(GUEST_BIN) --format hex); \
	  echo "Deploying with program_id=$$PROGRAM_ID"; \
	  lgs deploy

inspect:
	spel program-id $(GUEST_BIN)

fmt:
	cargo fmt --all
	cd ffi && cargo fmt --all

clippy:
	cargo clippy --all-targets -- -D warnings
	cd ffi && cargo clippy --all-targets -- -D warnings

test:
	RISC0_DEV_MODE=1 cargo test --workspace
	cd ffi && cargo test

clean:
	cargo clean
	cd ffi && cargo clean
	rm -f $(IDL) $(GUEST_BIN) $(FFI_SO)

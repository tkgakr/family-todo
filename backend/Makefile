ARCH := x86_64-unknown-linux-gnu

.PHONY: build-TodoHandler

build-TodoHandler:
	cargo lambda build --release --target $(ARCH) --bin main --output-format zip
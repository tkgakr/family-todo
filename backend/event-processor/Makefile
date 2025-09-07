build-lambda:
	cargo lambda build --release --output-format zip --target aarch64-unknown-linux-musl

build:
	cargo build --release -p event-processor
	cp ../target/release/event-processor bootstrap
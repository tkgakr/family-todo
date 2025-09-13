build-lambda:
	cargo lambda build --release --output-format zip --target x86_64-unknown-linux-musl

build-TodoEventProcessor:
	cargo build --release --target x86_64-unknown-linux-musl
	cp ../target/x86_64-unknown-linux-musl/release/event-processor bootstrap

build:
	cargo build --release -p event-processor
	cp ../target/release/event-processor bootstrap
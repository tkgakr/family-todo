build-lambda:
	cargo lambda build --release --output-format zip --target aarch64-unknown-linux-musl

build:
	cargo build --release -p command-handler
	cp ../target/release/command-handler bootstrap
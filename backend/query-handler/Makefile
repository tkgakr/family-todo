build-lambda:
	cargo lambda build --release --output-format zip --target aarch64-unknown-linux-musl

build:
	cargo build --release -p query-handler
	cp ../target/release/query-handler bootstrap
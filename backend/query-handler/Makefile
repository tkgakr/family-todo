build-lambda:
	cargo lambda build --release --output-format zip --target x86_64-unknown-linux-musl

build-TodoQueryHandler:
	cargo build --release --target x86_64-unknown-linux-musl
	cp ../target/x86_64-unknown-linux-musl/release/query-handler bootstrap

build:
	cargo build --release -p query-handler
	cp ../target/release/query-handler bootstrap
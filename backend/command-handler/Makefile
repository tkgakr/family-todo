build-lambda:
	cargo lambda build --release --output-format zip --target x86_64-unknown-linux-musl

build-TodoCommandHandler:
	cargo build --release --target x86_64-unknown-linux-musl
	cp ../target/x86_64-unknown-linux-musl/release/command-handler bootstrap

build:
	cargo build --release -p command-handler
	cp ../target/release/command-handler bootstrap
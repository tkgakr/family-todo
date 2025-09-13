build-lambda:
	cargo lambda build --release --output-format zip --target x86_64-unknown-linux-musl

build-TodoSnapshotManager:
	cargo build --release --target x86_64-unknown-linux-musl
	cp ../target/x86_64-unknown-linux-musl/release/snapshot-manager bootstrap

build:
	cargo build --release -p snapshot-manager
	cp ../target/release/snapshot-manager bootstrap
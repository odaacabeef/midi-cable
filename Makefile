.PHONY: build install clean test run

build:
	cargo build --release

install:
	cargo install --path .

clean:
	cargo clean

test:
	cargo test

run:
	cargo run --release

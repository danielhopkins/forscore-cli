.PHONY: build install clean

build:
	cargo build --release

install: build
	cp target/release/forscore ~/bin/

clean:
	cargo clean

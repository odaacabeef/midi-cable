.PHONY: build install

build:
	go build -o mc .

install: build
	mv mc $$HOME/go/bin/

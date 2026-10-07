# Browser build: WASM + a static page that prints over WebUSB.
#   make -C web          build into web/dist/
#   make -C web serve    build, then http://localhost:8000/ (WebUSB needs localhost or https)

WASM = ../target/wasm32-unknown-unknown/wasm/fec_receipt.wasm

dist: index.html samples/*
	cargo build --lib --profile wasm --target wasm32-unknown-unknown
	wasm-bindgen --target web --no-typescript --out-dir dist/pkg $(WASM)
	@if command -v wasm-opt >/dev/null; then wasm-opt -Oz -o dist/pkg/fec_receipt_bg.wasm dist/pkg/fec_receipt_bg.wasm; fi
	mkdir -p dist/samples
	cp index.html dist/
	cp samples/*.fec dist/samples/

serve: dist
	cd dist && python3 -m http.server 8000 --bind 127.0.0.1

.PHONY: dist serve

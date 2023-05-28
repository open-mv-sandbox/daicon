#!/bin/bash
set -e

cargo build --release -p daicon-web --example fetch --target wasm32-unknown-unknown
wasm-bindgen --out-name fetch \
  --out-dir dist/ \
  --target web target/wasm32-unknown-unknown/release/examples/fetch.wasm
cp ./crates/daicon-web/examples/fetch.html ./dist/fetch.html 

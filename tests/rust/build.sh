#!/bin/bash
set -ueo pipefail

cd preview1 && cargo build --target=wasm32-wasip1 && cd ..
cd preview2 && cargo build --target=wasm32-wasip2 && cd ..

cp preview1/target/wasm32-wasip1/debug/*.wasm testsuite/
cp preview2/target/wasm32-wasip2/debug/*.wasm testsuite/

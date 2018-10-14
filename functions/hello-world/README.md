# hello-world

## Building

```
cargo +nightly build --release &&
wasm-gc target/wasm32-unknown-unknown/release/hello_world.wasm &&
ipfs add -Q target/wasm32-unknown-unknown/release/hello_world.wasm &&
./set_function.sh xxx.eth
```

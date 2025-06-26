## Run Native

```
cargo run
```

## Run on web

### Using trunk

See [index.html](./index.html)

```
trunk serve
```

### Manually

See [manual.html](./manual.html)

```
cargo build --target wasm32-unknown-unknown
wasm-bindgen --out-dir ./out --target web ../../target/wasm32-unknown-unknown/debug/app.wasm
```

Then use a file server to host the manual.html:

```
miniserve .
```
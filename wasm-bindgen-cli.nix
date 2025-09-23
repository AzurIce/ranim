{
  buildWasmBindgenCli,
  fetchCrate,
  rustPlatform,
}:

buildWasmBindgenCli rec {
  src = fetchCrate {
    pname = "wasm-bindgen-cli";
    version = "0.2.103";
    hash = "sha256-ZMK/MpThET2b2uO+9gt9orjXbqLH5ZaoOQ9CAUU9PZY=";
  };

  cargoDeps = rustPlatform.fetchCargoVendor {
    inherit src;
    inherit (src) pname version;
    hash = "sha256-RYgCb25Cx5x7oLQPkj/2Il6IelvDti1kT+sizEDJETg=";
  };
}

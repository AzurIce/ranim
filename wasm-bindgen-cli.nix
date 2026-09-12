{
  buildWasmBindgenCli,
  fetchurl,
  rustPlatform,
}:

buildWasmBindgenCli rec {
  version = "0.2.128";

  src = fetchurl {
    name = "wasm-bindgen-cli-0.2.128.tar.gz";
    url = "https://static.crates.io/crates/wasm-bindgen-cli/wasm-bindgen-cli-${version}.crate";
    hash = "sha256-LikUDAToGDKQK3Dl03uc4b+oEcj+RWO+oI9234OIzyA=";
  };

  cargoDeps = rustPlatform.fetchCargoVendor {
    inherit src version;
    pname = "wasm-bindgen-cli";
    hash = "sha256-R1Tas33Ursy8kqsxguAkG0ZhNed2n5uFTAhw1l2qlLY=";
  };
}

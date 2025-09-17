{
  buildWasmBindgenCli,
  fetchCrate,
  rustPlatform,
}:

buildWasmBindgenCli rec {
  src = fetchCrate {
    pname = "wasm-bindgen-cli";
    version = "0.2.102";
    hash = "sha256-onynh0cGDko4RhNHfhvV9xlnyj+H7aPrwcBRIQm24so=";
  };

  cargoDeps = rustPlatform.fetchCargoVendor {
    inherit src;
    inherit (src) pname version;
    hash = "sha256-zNK9ihmX2TaHyXY8KuVXHPFGnCp8VouvLiZs7mBvbtM=";
  };
}

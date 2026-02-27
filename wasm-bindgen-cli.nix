{
  buildWasmBindgenCli,
  fetchCrate,
  rustPlatform,
}:

buildWasmBindgenCli rec {
  src = fetchCrate {
    pname = "wasm-bindgen-cli";
    version = "0.2.113";
    hash = "sha256-CWxeRhlO1i4Yq93OVLFDvJFIaBB7q2Ps0yqk+Euz+8w=";
  };

  cargoDeps = rustPlatform.fetchCargoVendor {
    inherit src;
    inherit (src) pname version;
    hash = "sha256-XmIx55PKfu+tVUGFC7MGF4AAYeV7z/p3KuLnY0bYMH8=";
  };
}

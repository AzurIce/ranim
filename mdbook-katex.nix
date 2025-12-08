{
  lib,
  rustPlatform,
  fetchCrate,
}:

rustPlatform.buildRustPackage rec {
  pname = "mdbook-katex";
  version = "0.10.0-alpha";

  src = fetchCrate {
    inherit pname version;
    hash = "sha256-F6ozNlN8umagAWr+xeA61uf+QOae/y6VnyzWKDsFIhk=";
  };

  cargoHash = "sha256-LUHVGEvE22ITlmpuI+8qGBPTa7q8YssiLSfQnvGM4hw=";

  meta = {
    description = "Preprocessor for mdbook, rendering LaTeX equations to HTML at build time";
    mainProgram = "mdbook-katex";
    homepage = "https://github.com/lzanini/${pname}";
    license = lib.licenses.mit;
    maintainers = with lib.maintainers; [
      lovesegfault
      matthiasbeyer
    ];
  };
}
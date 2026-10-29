// Browser compiler/renderer distribution: https://github.com/Myriad-Dreamin/typst.ts
const version = "0.7.0";
const npm = "https://cdn.jsdelivr.net/npm/@myriaddreamin/";
const fonts = "https://cdn.jsdelivr.net/gh/typst/typst-assets@v0.13.1/files/fonts/";
const main = "/notist-math.typ";
const template = `#set page(width: auto, height: auto, margin: 0pt, fill: none)
#set text(size: 12pt, font: "New Computer Modern")
#if sys.inputs.block == "true" {
  math.equation(block: true, eval(sys.inputs.math, mode: "math"))
} else {
  context [#metadata(here().position().y / 1pt) <notist-baseline>#box(eval(sys.inputs.math, mode: "math"))]
}`;

let engine;
let queue = Promise.resolve();
function load() {
  return engine ??= (async () => {
    const { createTypstCompiler, createTypstRenderer, loadFonts, TypstSnippet } =
      await import(`${npm}typst.ts@${version}/dist/esm/contrib/all-in-one-lite.bundle.js`);
    const compiler = createTypstCompiler();
    const renderer = createTypstRenderer();
    await Promise.all([
      compiler.init({
        getModule: () => `${npm}typst-ts-web-compiler@${version}/pkg/typst_ts_web_compiler_bg.wasm`,
        beforeBuild: [loadFonts([
          `${fonts}NewCMMath-Regular.otf`,
          `${fonts}NewCM10-Regular.otf`,
        ], { assets: false })],
      }),
      renderer.init({
        getModule: () => `${npm}typst-ts-renderer@${version}/pkg/typst_ts_renderer_bg.wasm`,
      }),
    ]);
    compiler.addSource(main, template);
    return { compiler, snippet: new TypstSnippet({ compiler, renderer }) };
  })().catch(error => { engine = undefined; throw error; });
}

export function compile(source, block, current) {
  // A shared compiler owns one source world. Serialize compilation and export;
  // a rejected formula must not block later formulas in the queue.
  const work = queue.then(async () => {
    if (!current()) return;
    const { compiler, snippet } = await load();
    if (!current()) return;
    const output = await compiler.runWithWorld(
      { mainFilePath: main, inputs: { math: source, block: String(block) } },
      async world => {
        // runWithWorld releases the WASM snapshot after this callback returns.
        const result = world.vector({ diagnostics: "full" });
        if (result.result && !block) {
          result.baseline = (await world.query({ selector: "<notist-baseline>", field: "value" }))[0];
        }
        return result;
      },
    );
    if (!output.result) {
      throw new Error(output.diagnostics?.map(diagnostic => diagnostic.message).join("; ") || "Typst compilation failed");
    }
    const svg = await snippet.svg({
      vectorData: output.result,
      data_selection: { body: true, defs: true, css: true, js: false },
    });
    return { svg, baseline: output.baseline };
  });
  queue = work.catch(() => {});
  return work;
}

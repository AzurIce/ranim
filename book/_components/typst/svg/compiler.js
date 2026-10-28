// The package registry uses synchronous file access. Keep it and WASM off the UI thread.
const version = "0.7.0";
const npm = "https://cdn.jsdelivr.net/npm/@myriaddreamin/";
const fonts = "https://cdn.jsdelivr.net/gh/typst/typst-assets@v0.13.1/files/fonts/";
const cjk = "https://cdn.jsdelivr.net/gh/typst/typst-dev-assets@v0.13.1/files/fonts/NotoSerifCJKsc-Regular.otf";
const main = "/notist-svg.typ";
const defaults = `#set page(width: auto, height: auto, margin: 0pt, fill: none)
#set text(size: 12pt, font: ("Libertinus Serif", "Noto Serif CJK SC"))
`;

let engine;
let queue = Promise.resolve();
function load() {
  return engine ??= (async () => {
    const modules = `${npm}typst.ts@${version}/dist/esm/`;
    const [{ createTypstCompiler }, { createTypstRenderer }, { loadFonts }, { TypstSnippet }] = await Promise.all([
      import(`${modules}compiler.mjs`),
      import(`${modules}renderer.mjs`),
      import(`${modules}options.init.mjs`),
      import(`${modules}contrib/snippet.mjs`),
    ]);
    const compiler = createTypstCompiler();
    const renderer = createTypstRenderer();
    await Promise.all([
      compiler.init({
        getWrapper: () => import(`${npm}typst-ts-web-compiler@${version}/pkg/typst_ts_web_compiler.mjs`),
        getModule: () => `${npm}typst-ts-web-compiler@${version}/pkg/typst_ts_web_compiler_bg.wasm`,
        beforeBuild: [
          ...TypstSnippet.fetchPackageRegistry().provides,
          loadFonts([`${fonts}NewCMMath-Regular.otf`, `${fonts}LibertinusSerif-Regular.otf`, cjk], { assets: false }),
        ],
      }),
      renderer.init({
        getWrapper: () => import(`${npm}typst-ts-renderer@${version}/pkg/typst_ts_renderer.mjs`),
        getModule: () => `${npm}typst-ts-renderer@${version}/pkg/typst_ts_renderer_bg.wasm`,
      }),
    ]);
    return { compiler, snippet: new TypstSnippet({ compiler, renderer }) };
  })().catch(error => { engine = undefined; throw error; });
}

async function compile(source) {
  const { compiler, snippet } = await load();
  compiler.addSource(main, defaults + source);
  const output = await compiler.runWithWorld(
    { mainFilePath: main, inputs: {} },
    async world => world.vector({ diagnostics: "full" }),
  );
  if (!output.result) {
    throw new Error(output.diagnostics?.map(diagnostic => diagnostic.message).join("; ") || "Typst compilation failed");
  }
  return snippet.svg({
    vectorData: output.result,
    data_selection: { body: true, defs: true, css: true, js: false },
  });
}

self.onmessage = ({ data: { id, source } }) => {
  // Compilation mutates one source world; failures release the queue for later requests.
  queue = queue.then(async () => {
    try {
      self.postMessage({ id, svg: await compile(source) });
    } catch (error) {
      self.postMessage({ id, error: error.message || String(error) });
    }
  });
};

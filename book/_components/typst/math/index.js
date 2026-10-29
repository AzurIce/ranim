import { compile } from "./compiler.js";

export default class TypstMath extends HTMLElement {
  static observedAttributes = ["notist-text", "notist-content"];
  #revision = 0;
  constructor() {
    super();
    this.attachShadow({ mode: "open" }).innerHTML =
      `<style>:host { display: inline; } :host([notist-content="block"]) { display: block; text-align: center; margin: 1em 0; } svg { overflow: visible; } .error { color: #a21; font: inherit; margin-left: .4em; }</style><span class="formula"></span><span class="error" role="status"></span>`;
  }
  connectedCallback() { this.update(); }
  disconnectedCallback() { this.#revision++; }
  attributeChangedCallback() { if (this.isConnected) this.update(); }
  async update() {
    const revision = ++this.#revision;
    const source = this.getAttribute("notist-text") ?? "";
    const block = this.getAttribute("notist-content") === "block";
    const target = this.shadowRoot.querySelector(".formula");
    const message = this.shadowRoot.querySelector(".error");
    const current = () => this.isConnected && revision === this.#revision;
    target.textContent = source;
    message.textContent = "";
    if (!source.trim()) return;
    try {
      const output = await compile(source, block, current);
      if (!current()) return;
      const document = new DOMParser().parseFromString(output.svg, "image/svg+xml");
      const svg = document.documentElement;
      if (svg.localName !== "svg") throw new Error("Typst returned invalid SVG");
      // The formula has one accessible label. The renderer's invisible text
      // selection overlay would distort the visible bounds of math.
      svg.querySelectorAll("foreignObject").forEach(node => node.remove());
      svg.setAttribute("role", "img");
      svg.setAttribute("aria-label", source);
      target.replaceChildren(svg);
      // Inline equations can extend beyond Typst's paragraph/page bounds.
      // Include their ink and keep the measured Typst baseline in the text line.
      const box = svg.getBBox();
      const page = svg.viewBox.baseVal;
      const left = Math.min(0, box.x - .5);
      const top = Math.min(0, box.y - .5);
      const width = Math.max(page.width, box.x + box.width + .5) - left;
      const height = Math.max(page.height, box.y + box.height + .5) - top;
      svg.setAttribute("viewBox", `${left} ${top} ${width} ${height}`);
      svg.style.width = `${width / 12}em`;
      svg.style.height = `${height / 12}em`;
      if (!block) svg.style.verticalAlign = `${(output.baseline - top - height) / 12}em`;
    } catch (error) {
      if (!current()) return;
      target.textContent = source;
      message.textContent = `Math error: ${error.message}`;
    }
  }
}

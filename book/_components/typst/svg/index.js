import { compile } from "./runtime.js";

export default class TypstSvg extends HTMLElement {
  static observedAttributes = ["notist-source"];
  #revision = 0;
  #renderedRevision = 0;
  constructor() {
    super();
    this.attachShadow({ mode: "open" }).innerHTML = `<style>
      :host { display: block; min-width: 0; }
      .drawing { min-width: 0; text-align: center; }
      svg { display: block; max-width: 100%; max-height: 85vh; height: auto; margin-inline: auto; }
      .source, .error { white-space: pre-wrap; overflow-wrap: anywhere; font: inherit; }
      .error { color: #a21; }
      [hidden] { display: none; }
    </style><div class="drawing"></div><pre class="source"></pre><div class="error" role="status"></div>`;
  }
  connectedCallback() { this.update(); }
  disconnectedCallback() { this.#revision++; }
  attributeChangedCallback() { if (this.isConnected) this.update(); }
  get renderedSvg() {
    return this.isConnected && this.#renderedRevision === this.#revision
      ? this.shadowRoot.querySelector("svg")
      : null;
  }
  async update() {
    const revision = ++this.#revision;
    const source = this.getAttribute("notist-source") ?? "";
    const drawing = this.shadowRoot.querySelector(".drawing");
    const fallback = this.shadowRoot.querySelector(".source");
    const message = this.shadowRoot.querySelector(".error");
    drawing.replaceChildren();
    fallback.textContent = source;
    fallback.hidden = false;
    message.textContent = "";
    if (!source.trim()) return;
    const current = () => this.isConnected && revision === this.#revision;
    try {
      const result = await compile(source);
      if (!current()) return;
      const parsed = new DOMParser().parseFromString(result, "image/svg+xml");
      const svg = parsed.documentElement;
      if (svg.localName !== "svg") throw new Error("Typst returned invalid SVG");
      // Keep the vector artwork, without the renderer's text-selection HTML overlay.
      svg.querySelectorAll("foreignObject").forEach(node => node.remove());
      svg.setAttribute("role", "img");
      svg.setAttribute("aria-label", source);
      drawing.replaceChildren(svg);
      fallback.hidden = true;
      this.#renderedRevision = revision;
    } catch (error) {
      if (!current()) return;
      message.textContent = `Typst error: ${error.message}`;
    }
  }
}

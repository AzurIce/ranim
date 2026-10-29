import styles from "./style.js";

let library;
function load() {
  return library ??= import("https://cdn.jsdelivr.net/npm/pele@1.6.0/dist/pele.js")
    .catch(error => { library = undefined; throw error; });
}

export default class Diagram extends HTMLElement {
  static observedAttributes = ["notist-source", "notist-theme"];
  #revision = 0;
  #renderedRevision = 0;
  #mounted;
  #target;
  #media;
  #themeObserver;
  #updateTheme = () => {
    const requested = this.getAttribute("notist-theme");
    const schemes = this.ownerDocument.defaultView.getComputedStyle(this).colorScheme.split(/\s+/);
    const dark = schemes.includes("dark"), light = schemes.includes("light");
    const automatic = dark !== light ? dark : this.#media.matches;
    this.dataset.peleScheme = (requested === "dark" || (requested !== "light" && automatic)) ? "dark" : "light";
    // Hiding and revealing within one frame may produce no resize notification.
    // Refresh also picks up deferred layouts and inherited font changes.
    this.#mounted?.refresh();
  };
  constructor() {
    super();
    const root = this.attachShadow({ mode: "open" });
    root.innerHTML = `<style>${styles}</style><div class="diagram"></div>`;
    this.#target = root.querySelector(".diagram");
  }
  connectedCallback() {
    this.#media = this.ownerDocument.defaultView.matchMedia("(prefers-color-scheme: dark)");
    this.#media.addEventListener("change", this.#updateTheme);
    this.#themeObserver = new MutationObserver(this.#updateTheme);
    for (let ancestor = this; ancestor; ancestor = ancestor.parentElement ?? ancestor.getRootNode().host) {
      this.#themeObserver.observe(ancestor, { attributes: true, attributeFilter: ["data-theme", "class", "style"] });
    }
    this.#updateTheme();
    this.update();
  }
  disconnectedCallback() {
    this.#revision++;
    this.#media?.removeEventListener("change", this.#updateTheme);
    this.#themeObserver?.disconnect();
    this.#mounted?.destroy();
    this.#mounted = undefined;
  }
  attributeChangedCallback(name) {
    if (!this.isConnected || !this.#media) return;
    if (name === "notist-theme") this.#updateTheme();
    else this.update();
  }
  get renderedSvg() {
    return this.isConnected && this.#renderedRevision === this.#revision && this.#mounted?.error === undefined
      ? this.#target.querySelector("svg.pele")
      : null;
  }
  async update() {
    const revision = ++this.#revision;
    const source = this.getAttribute("notist-source") ?? "";
    try {
      const { mount } = await load();
      if (!this.isConnected || revision !== this.#revision) return;
      // Pele owns resizing, font remeasurement, zoom and source-error recovery.
      // Its mount is synchronous; only the shared library load can become stale.
      const options = {
        onRender: () => {
          // A hidden container can defer an update until its width is available.
          if (this.isConnected && revision === this.#revision) this.#renderedRevision = revision;
        },
      };
      if (this.#mounted) this.#mounted.update(source, options);
      else this.#mounted = mount(this.#target, source, options);
    } catch (error) {
      if (!this.isConnected || revision !== this.#revision) return;
      this.#mounted?.destroy();
      this.#mounted = undefined;
      const message = this.ownerDocument.createElement("pre");
      message.className = "load-error";
      message.setAttribute("role", "alert");
      message.textContent = `Diagram error: ${error.message}`;
      this.#target.replaceChildren(message);
    }
  }
}

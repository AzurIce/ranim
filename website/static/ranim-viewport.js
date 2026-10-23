const modules = new Map();

function loadProjectModule(url) {
  if (!modules.has(url)) {
    const pending = import(url).then(async (module) => {
      await module.default();
      return module;
    });
    modules.set(url, pending);
    pending.catch(() => modules.delete(url));
  }
  return modules.get(url);
}

/** A canvas session and host playback clock, independent of any controls. */
export class RanimViewport extends HTMLElement {
  static observedAttributes = ["src", "scene"];

  constructor() {
    super();
    this.attachShadow({ mode: "open" }).innerHTML = `
      <style>
        :host { display: block; min-width: 0; color-scheme: dark; }
        .viewport {
          overflow: hidden; border: 1px solid #ffffff12; border-radius: 12px;
          background: #333; box-shadow: 0 2px 4px #00000008, 0 10px 30px -12px #00000040;
        }
        canvas { display: block; width: 100%; aspect-ratio: 16 / 9; background: #333; }
        .status {
          margin: 0; padding: 10px 16px; border-top: 1px solid #ffffff0d;
          background: #1b1e25; color: #b7c2d1;
          font: 12px/1.6 Inter, ui-sans-serif, system-ui, sans-serif;
          overflow-wrap: anywhere;
        }
        .status:empty { display: none; }
      </style>
      <div class="viewport" part="viewport">
        <canvas part="canvas" width="1280" height="720" aria-label="Animation preview"></canvas>
        <p class="status" role="status" aria-live="polite"></p>
      </div>`;
    this._canvas = this.shadowRoot.querySelector("canvas");
    this._status = this.shadowRoot.querySelector(".status");
    this._session = null;
    this._generation = 0;
    this._raf = 0;
    this._duration = 0;
    this._time = 0;
    this._playing = false;
    this._dirty = false;
    this._resizeObserver = new ResizeObserver(() => this._invalidate());
    this._onResize = () => this._invalidate();
  }

  connectedCallback() {
    window.addEventListener("resize", this._onResize);
    this._load();
  }

  disconnectedCallback() {
    ++this._generation;
    window.removeEventListener("resize", this._onResize);
    this._release();
  }

  attributeChangedCallback() {
    if (this.isConnected) this._load();
  }

  get ready() { return this._session !== null; }
  get duration() { return this._duration; }
  get currentTime() { return this._position(performance.now()); }
  set currentTime(seconds) { this.seek(seconds); }
  get paused() { return !this._playing; }

  play() {
    if (!this._session || this._playing || this._duration <= 0) return;
    if (this._time >= this._duration) this._time = 0;
    this._anchorTime = this._time;
    this._anchorNow = performance.now();
    this._playing = true;
    this._schedule();
    this.dispatchEvent(new Event("play"));
  }

  pause() {
    if (!this._playing) return;
    this._time = this.currentTime;
    this._playing = false;
    this._invalidate();
    this.dispatchEvent(new Event("pause"));
  }

  seek(seconds) {
    if (!this._session) return;
    if (!Number.isFinite(seconds)) throw new TypeError("Preview time must be finite");
    this._time = Math.max(0, Math.min(seconds, this._duration));
    this._anchorTime = this._time;
    this._anchorNow = performance.now();
    this._invalidate();
    this.dispatchEvent(new Event("timeupdate"));
  }

  async _load() {
    const generation = ++this._generation;
    this._release();
    const src = this.getAttribute("src");
    if (!src) {
      this._status.textContent = "";
      return;
    }
    this._status.textContent = "Loading preview…";
    this.dispatchEvent(new Event("loading"));
    // A stale initialization may still be awaiting the GPU. Give each load
    // its own canvas, so it cannot reconfigure a newer session's surface.
    const canvas = this._canvas.cloneNode();
    this._canvas.replaceWith(canvas);
    this._canvas = canvas;
    let project;
    try {
      const module = await loadProjectModule(new URL(src, this.baseURI).href);
      if (generation !== this._generation) return;
      project = module.app();
      const scene = this.getAttribute("scene") || project.sceneNames().split("\n")[0];
      const session = await project.createPreview(canvas, scene);
      if (generation !== this._generation) {
        session.dispose();
        session.free();
        return;
      }
      this._session = session;
      this._duration = session.duration;
      this._status.textContent = "";
      this._resizeObserver.observe(canvas);
      this._invalidate();
      this.dispatchEvent(new CustomEvent("ready", { detail: { scene, duration: this._duration } }));
    } catch (error) {
      if (generation === this._generation) this._fail(error);
    } finally {
      // createPreview borrows this handle until its Promise settles.
      project?.free();
    }
  }

  _release() {
    cancelAnimationFrame(this._raf);
    this._raf = 0;
    this._resizeObserver.disconnect();
    if (this._session) {
      const session = this._session;
      this._session = null;
      try {
        session.dispose();
        session.free();
      } catch (error) {
        // A WASM panic can leave its handle borrowed. Still stop the UI and
        // display the original error instead of throwing again during cleanup.
        console.warn("Unable to release preview", error);
      }
    }
    this._playing = false;
    this._dirty = false;
    this._time = 0;
    this._duration = 0;
    this.dispatchEvent(new Event("emptied"));
  }

  _position(now) {
    return this._playing
      ? Math.min(this._duration, this._anchorTime + (now - this._anchorNow) / 1000)
      : this._time;
  }

  _invalidate() {
    this._dirty = true;
    this._schedule();
  }

  _schedule() {
    if (!this._raf && this._session && this.isConnected) {
      this._raf = requestAnimationFrame((now) => this._tick(now));
    }
  }

  _tick(now) {
    this._raf = 0;
    this._time = this._position(now);
    try {
      this._dirty = !this._draw();
    } catch (error) {
      this._fail(error);
      return;
    }
    const ended = this._playing && this._time >= this._duration;
    if (ended) this._playing = false;
    this.dispatchEvent(new Event("timeupdate"));
    if (ended) {
      this.dispatchEvent(new Event("pause"));
      this.dispatchEvent(new Event("ended"));
    }
    if (this._playing || this._dirty) this._schedule();
  }

  _draw() {
    const cssWidth = this._canvas.getBoundingClientRect().width;
    // Hidden tabs will be invalidated by ResizeObserver when shown again.
    if (cssWidth <= 0) return true;
    const width = Math.max(1, Math.min(1280, Math.round(cssWidth * (window.devicePixelRatio || 1))));
    const height = Math.max(1, Math.round(width * 9 / 16));
    if (width !== this._canvas.width || height !== this._canvas.height) {
      this._session.resize(width, height);
    }
    return this._session.renderAt(this._time);
  }

  _fail(error) {
    this._release();
    this._status.textContent = `Unable to load preview: ${error instanceof Error ? error.message : error}`;
    this.dispatchEvent(new CustomEvent("error", { detail: error }));
  }
}

customElements.define("ranim-viewport", RanimViewport);

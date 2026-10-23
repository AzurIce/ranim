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

function formatTime(seconds) {
  const ticks = Math.floor(seconds * 100);
  return `${Math.floor(ticks / 6000)}:${String(Math.floor(ticks / 100) % 60).padStart(2, "0")}.${String(ticks % 100).padStart(2, "0")}`;
}

/** A canvas and playback controls. The host clock stays in JavaScript. */
export class RanimPlayer extends HTMLElement {
  static observedAttributes = ["src", "scene"];

  constructor() {
    super();
    this.attachShadow({ mode: "open" }).innerHTML = `
      <style>
        :host {
          display: block; container-type: inline-size; color-scheme: dark;
          --accent: var(--ranim-accent, #70d3e8);
          --surface: var(--ranim-controls-background, #1b1e25);
          --track: #ffffff24;
          font-family: Inter, ui-sans-serif, system-ui, sans-serif;
        }
        * { box-sizing: border-box; }
        .player {
          overflow: hidden; isolation: isolate; border: 1px solid #ffffff12;
          border-radius: 12px; background: #333;
          box-shadow: 0 2px 4px #00000008, 0 10px 30px -12px #00000040;
        }
        canvas { display: block; width: 100%; aspect-ratio: 16 / 9; background: #333; }
        .controls {
          display: grid; grid-template-columns: 34px minmax(32px, 1fr) auto;
          align-items: center; gap: 14px; padding: 11px 16px;
          border-top: 1px solid #ffffff0d; background: var(--surface); color: #edf1f7;
        }
        button {
          display: grid; place-items: center; width: 34px; height: 34px; padding: 0;
          border: 1px solid #70d3e830; border-radius: 9px;
          background: #70d3e812; color: var(--accent); cursor: pointer;
          transition: background 120ms, border-color 120ms, transform 120ms;
        }
        button:hover:not(:disabled) { background: #70d3e827; border-color: #70d3e860; }
        button:active:not(:disabled) { transform: scale(.94); }
        button:focus-visible { outline: 2px solid var(--accent); outline-offset: 3px; }
        button:disabled { opacity: .4; cursor: default; }
        button svg { width: 16px; height: 16px; fill: currentColor; }
        .pause-icon, button[data-playing] .play-icon { display: none; }
        button[data-playing] .pause-icon { display: block; }
        input {
          appearance: none; width: 100%; min-width: 0; height: 28px; margin: 0;
          background: transparent; cursor: pointer; --progress: 0%;
        }
        input::-webkit-slider-runnable-track {
          height: 4px; border-radius: 4px;
          background: linear-gradient(to right, var(--accent) var(--progress), var(--track) var(--progress));
        }
        input::-moz-range-track {
          height: 4px; border-radius: 4px;
          background: linear-gradient(to right, var(--accent) var(--progress), var(--track) var(--progress));
        }
        input::-webkit-slider-thumb {
          appearance: none; width: 10px; height: 10px; margin-top: -3px;
          border: 0; border-radius: 50%; background: var(--accent);
          box-shadow: 0 0 0 3px #70d3e818; transition: box-shadow 120ms, transform 120ms;
        }
        input::-moz-range-thumb {
          width: 10px; height: 10px; border: 0; border-radius: 50%; background: var(--accent);
          box-shadow: 0 0 0 3px #70d3e818; transition: box-shadow 120ms, transform 120ms;
        }
        input:hover::-webkit-slider-thumb { box-shadow: 0 0 0 5px #70d3e825; }
        input:hover::-moz-range-thumb { box-shadow: 0 0 0 5px #70d3e825; }
        input:focus-visible { outline: 2px solid var(--accent); outline-offset: 4px; border-radius: 4px; }
        input:disabled { opacity: .35; cursor: default; }
        output {
          color: #a6afbe; font: 11px ui-monospace, SFMono-Regular, Consolas, monospace;
          font-variant-numeric: tabular-nums; white-space: nowrap; letter-spacing: -.2px;
        }
        .status {
          margin: 0; padding: 10px 16px; border-top: 1px solid #ffffff0d;
          background: var(--surface); color: #b7c2d1; font-size: 12px; line-height: 1.6;
          overflow-wrap: anywhere;
        }
        .status:empty { display: none; }
        @container (max-width: 360px) {
          .controls { gap: 10px; padding: 10px 12px; }
        }
        @container (max-width: 260px) {
          .controls { grid-template-columns: 34px minmax(32px, 1fr); }
          output { grid-column: 2; text-align: right; }
        }
        @media (prefers-reduced-motion: reduce) {
          button { transition: none; }
          input::-webkit-slider-thumb { transition: none; }
          input::-moz-range-thumb { transition: none; }
        }
      </style>
      <div class="player" part="player">
        <canvas part="canvas" width="1280" height="720" aria-label="Animation preview"></canvas>
        <div class="controls" part="controls">
          <button type="button" aria-label="Play" title="Play" disabled>
            <svg class="play-icon" viewBox="0 0 20 20" aria-hidden="true"><path d="M5 3.8a1 1 0 0 1 1.5-.86l9.2 5.32a2 2 0 0 1 0 3.48L6.5 17.06A1 1 0 0 1 5 16.2Z"/></svg>
            <svg class="pause-icon" viewBox="0 0 20 20" aria-hidden="true"><rect x="5" y="3" width="3.5" height="14" rx="1"/><rect x="11.5" y="3" width="3.5" height="14" rx="1"/></svg>
          </button>
          <input type="range" min="0" max="0" step="any" value="0" aria-label="Seek" disabled>
          <output aria-live="off">0:00.00 / 0:00.00</output>
        </div>
        <p class="status" role="status" aria-live="polite"></p>
      </div>`;
    this._canvas = this.shadowRoot.querySelector("canvas");
    this._button = this.shadowRoot.querySelector("button");
    this._slider = this.shadowRoot.querySelector("input");
    this._output = this.shadowRoot.querySelector("output");
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
    this._button.addEventListener("click", () => this.paused ? this.play() : this.pause());
    this._slider.addEventListener("input", () => this.seek(Number(this._slider.value)));
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
    this._updateControls();
    this._schedule();
    this.dispatchEvent(new Event("play"));
  }

  pause() {
    if (!this._playing) return;
    this._time = this.currentTime;
    this._playing = false;
    this._invalidate();
    this._updateControls();
    this.dispatchEvent(new Event("pause"));
  }

  seek(seconds) {
    if (!this._session) return;
    if (!Number.isFinite(seconds)) throw new TypeError("Preview time must be finite");
    this._time = Math.max(0, Math.min(seconds, this._duration));
    this._anchorTime = this._time;
    this._anchorNow = performance.now();
    this._updateControls();
    this._invalidate();
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
      this._slider.max = String(this._duration);
      this._button.disabled = this._duration <= 0;
      this._slider.disabled = this._duration <= 0;
      this._status.textContent = "";
      this._resizeObserver.observe(canvas);
      this._updateControls();
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
    this._button.disabled = true;
    this._slider.disabled = true;
    this._slider.max = "0";
    this._updateControls();
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
    this._updateControls();
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

  _updateControls() {
    const label = this._playing ? "Pause" : "Play";
    this._button.setAttribute("aria-label", label);
    this._button.title = label;
    this._button.toggleAttribute("data-playing", this._playing);
    this._slider.value = String(this._time);
    this._slider.style.setProperty("--progress", `${this._duration > 0 ? this._time / this._duration * 100 : 0}%`);
    this._slider.setAttribute("aria-valuetext", `${formatTime(this._time)} of ${formatTime(this._duration)}`);
    this._output.textContent = `${formatTime(this._time)} / ${formatTime(this._duration)}`;
  }

  _fail(error) {
    this._release();
    this._status.textContent = `Unable to load preview: ${error instanceof Error ? error.message : error}`;
    this.dispatchEvent(new CustomEvent("error", { detail: error }));
  }
}

customElements.define("ranim-player", RanimPlayer);

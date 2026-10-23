const stateEvents = ["loading", "ready", "timeupdate", "play", "pause", "ended", "emptied", "error"];

function formatTime(seconds) {
  const ticks = Math.floor(seconds * 100);
  return `${Math.floor(ticks / 6000)}:${String(Math.floor(ticks / 100) % 60).padStart(2, "0")}.${String(ticks % 100).padStart(2, "0")}`;
}

/** Playback controls bound to a viewport by ID or through the target property. */
export class RanimControls extends HTMLElement {
  static observedAttributes = ["for"];

  constructor() {
    super();
    this.attachShadow({ mode: "open" }).innerHTML = `
      <style>
        :host {
          display: block; min-width: 0; container-type: inline-size; color-scheme: dark;
          --accent: var(--ranim-accent, #70d3e8);
          --surface: var(--ranim-controls-background, #1b1e25);
          --track: #ffffff24;
          font-family: Inter, ui-sans-serif, system-ui, sans-serif;
        }
        * { box-sizing: border-box; }
        .controls {
          display: grid; grid-template-columns: 34px minmax(32px, 1fr) auto;
          align-items: center; gap: 14px; padding: 11px 16px;
          border: 1px solid #ffffff12; border-radius: 12px;
          background: var(--surface); color: #edf1f7;
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
        <div class="controls" part="controls">
          <button type="button" aria-label="Play" title="Play" disabled>
            <svg class="play-icon" viewBox="0 0 20 20" aria-hidden="true"><path d="M5 3.8a1 1 0 0 1 1.5-.86l9.2 5.32a2 2 0 0 1 0 3.48L6.5 17.06A1 1 0 0 1 5 16.2Z"/></svg>
            <svg class="pause-icon" viewBox="0 0 20 20" aria-hidden="true"><rect x="5" y="3" width="3.5" height="14" rx="1"/><rect x="11.5" y="3" width="3.5" height="14" rx="1"/></svg>
          </button>
          <input type="range" min="0" max="0" step="any" value="0" aria-label="Seek" disabled>
          <output aria-live="off">0:00.00 / 0:00.00</output>
        </div>`;
    this._button = this.shadowRoot.querySelector("button");
    this._slider = this.shadowRoot.querySelector("input");
    this._output = this.shadowRoot.querySelector("output");
    this._target = null;
    this._assignedTarget = undefined;
    this._observer = null;
    this._onStateChange = () => this._updateControls();
    this._button.addEventListener("click", () => {
      if (this._target.paused) this._target.play();
      else this._target.pause();
    });
    this._slider.addEventListener("input", () => this._target?.seek(Number(this._slider.value)));
  }

  connectedCallback() {
    // Preserve property assignments made before the element was upgraded.
    if (Object.hasOwn(this, "target")) {
      const target = this.target;
      delete this.target;
      this.target = target;
    } else {
      this._watchTarget();
    }
  }

  disconnectedCallback() {
    this._observer?.disconnect();
    this._observer = null;
    this._bind(null);
  }

  attributeChangedCallback() {
    this._assignedTarget = undefined;
    this._watchTarget();
  }

  get target() { return this._assignedTarget !== undefined ? this._assignedTarget : this._target; }
  set target(target) {
    this._assignedTarget = target;
    this._watchTarget();
  }

  _watchTarget() {
    this._observer?.disconnect();
    this._observer = null;
    if (!this.isConnected) return;
    if (this._assignedTarget !== undefined) {
      this._bind(this._assignedTarget);
      return;
    }
    const id = this.getAttribute("for");
    if (!id) {
      this._bind(null);
      return;
    }
    const root = this.getRootNode();
    const resolve = () => this._bind(root.getElementById(id));
    resolve();
    // Track late insertion, replacement, and ID changes in this DOM scope.
    this._observer = new MutationObserver(resolve);
    this._observer.observe(root, { childList: true, subtree: true, attributes: true, attributeFilter: ["id"] });
  }

  _bind(target) {
    if (target !== this._target) {
      for (const name of stateEvents) this._target?.removeEventListener(name, this._onStateChange);
      this._target = target;
      for (const name of stateEvents) target?.addEventListener(name, this._onStateChange);
    }
    this._updateControls();
  }

  _updateControls() {
    const target = this._target;
    const enabled = target?.ready === true && target.duration > 0;
    const playing = enabled && !target.paused;
    const duration = target?.duration ?? 0;
    const time = target?.currentTime ?? 0;
    const label = playing ? "Pause" : "Play";
    this._button.disabled = !enabled;
    this._slider.disabled = !enabled;
    this._button.setAttribute("aria-label", label);
    this._button.title = label;
    this._button.toggleAttribute("data-playing", playing);
    this._slider.max = String(duration);
    this._slider.value = String(time);
    this._slider.style.setProperty("--progress", `${duration > 0 ? time / duration * 100 : 0}%`);
    this._slider.setAttribute("aria-valuetext", `${formatTime(time)} of ${formatTime(duration)}`);
    this._output.textContent = `${formatTime(time)} / ${formatTime(duration)}`;
  }
}

customElements.define("ranim-controls", RanimControls);

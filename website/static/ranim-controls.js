import { TimelinePanel } from "./ranim-timeline.js";
import { previewTheme } from "./ranim-theme.js";

const stateEvents = ["loading", "ready", "timeupdate", "play", "pause", "ended", "emptied", "error", "volumechange"];
const speakerIcon = `<svg viewBox="0 0 20 20" aria-hidden="true"><path d="M3 7h3l4-3v12l-4-3H3z"/><g class="sound-waves" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"><path d="M13 7a4 4 0 0 1 0 6M15.5 4.5a7.5 7.5 0 0 1 0 11"/></g><path class="mute-mark" d="m13 8 4 4m0-4-4 4" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>`;

function formatTime(seconds) {
  const ticks = Math.floor(seconds * 100);
  return `${Math.floor(ticks / 6000)}:${String(Math.floor(ticks / 100) % 60).padStart(2, "0")}.${String(ticks % 100).padStart(2, "0")}`;
}

/** Playback controls bound to a viewport by ID or through the target property. */
export class RanimControls extends HTMLElement {
  static observedAttributes = ["for", "timeline-expanded"];

  constructor() {
    super();
    this.attachShadow({ mode: "open" }).innerHTML = `
      <style>
        ${previewTheme}
        :host {
          display: block; min-width: 0; container-type: inline-size;
          --track: color-mix(in srgb, var(--text) 15%, var(--surface));
        }
        * { box-sizing: border-box; }
        .controls {
          display: grid; grid-template-columns: 34px minmax(32px, 1fr) auto 28px 28px;
          align-items: center; gap: 12px; padding: 8px 12px;
          border: 1px solid var(--border); border-radius: var(--radius);
          background: var(--surface);
        }
        button {
          display: grid; place-items: center; width: 34px; height: 34px; padding: 0;
          border: 1px solid transparent; border-radius: 4px;
          background: transparent; color: var(--muted); cursor: pointer;
          transition: background 120ms, color 120ms;
        }
        button:hover:not(:disabled) { background: var(--hover); color: var(--text); }
        button:active:not(:disabled) { background: var(--accent-soft); }
        button:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
        .playback { color: var(--accent); background: var(--accent-soft); }
        .playback:hover:not(:disabled) { background: color-mix(in srgb, var(--accent) 15%, var(--surface)); color: var(--accent); }
        button:disabled { opacity: .4; cursor: default; }
        button svg { width: 16px; height: 16px; fill: currentColor; }
        .timeline-toggle, .volume-toggle, .mute { width: 28px; height: 28px; }
        .mute-mark, [data-silent] .sound-waves { display: none; }
        [data-silent] .mute-mark { display: block; }
        .volume-popup {
          position: fixed; inset: auto; width: 208px; margin: 0; padding: 12px;
          border: 1px solid var(--border); border-radius: var(--radius);
          background: var(--surface); color: var(--text); box-shadow: 0 4px 16px #00000014;
          font: inherit; font-size: 12px;
        }
        .volume-popup:popover-open { display: flex; align-items: center; gap: 10px; }
        .volume-popup input { flex: 1; }
        .volume-value { min-width: 34px; color: var(--muted); text-align: right; font-variant-numeric: tabular-nums; }
        .timeline-toggle svg { fill: none; transition: transform 120ms; }
        :host([timeline-expanded]) .timeline-toggle svg { transform: rotate(180deg); }
        :host([timeline-expanded]) .controls { border-bottom: 0; border-radius: var(--radius) var(--radius) 0 0; }
        .timeline { overflow: hidden; border: 1px solid var(--border); border-top: 0; border-radius: 0 0 var(--radius) var(--radius); }
        .timeline[hidden] { display: none; }
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
          box-shadow: none; transition: box-shadow 120ms;
        }
        input::-moz-range-thumb {
          width: 10px; height: 10px; border: 0; border-radius: 50%; background: var(--accent);
          box-shadow: none; transition: box-shadow 120ms;
        }
        input:hover::-webkit-slider-thumb { box-shadow: 0 0 0 4px color-mix(in srgb, var(--accent) 12%, transparent); }
        input:hover::-moz-range-thumb { box-shadow: 0 0 0 4px color-mix(in srgb, var(--accent) 12%, transparent); }
        input:focus-visible { outline: 2px solid var(--accent); outline-offset: 4px; border-radius: 4px; }
        input:disabled { opacity: .35; cursor: default; }
        output {
          color: var(--muted); font: 11px var(--mono);
          font-variant-numeric: tabular-nums; white-space: nowrap; letter-spacing: -.2px;
        }
        @container (max-width: 360px) {
          .controls { gap: 8px; padding: 10px 12px; }
        }
        @container (max-width: 260px) {
          .controls { grid-template-columns: 34px minmax(32px, 1fr) 28px 28px; }
          .time { grid-column: 2 / 5; grid-row: 2; text-align: right; }
          .volume-toggle { grid-column: 3; grid-row: 1; }
          .timeline-toggle { grid-column: 4; grid-row: 1; }
        }
        @media (prefers-reduced-motion: reduce) {
          button { transition: none; }
          .timeline-toggle svg { transition: none; }
          input::-webkit-slider-thumb { transition: none; }
          input::-moz-range-thumb { transition: none; }
        }
      </style>
        <div class="controls" part="controls">
          <button class="playback" type="button" aria-label="Play" title="Play" disabled>
            <svg class="play-icon" viewBox="0 0 20 20" aria-hidden="true"><path d="M5 3.8a1 1 0 0 1 1.5-.86l9.2 5.32a2 2 0 0 1 0 3.48L6.5 17.06A1 1 0 0 1 5 16.2Z"/></svg>
            <svg class="pause-icon" viewBox="0 0 20 20" aria-hidden="true"><rect x="5" y="3" width="3.5" height="14" rx="1"/><rect x="11.5" y="3" width="3.5" height="14" rx="1"/></svg>
          </button>
          <input type="range" min="0" max="0" step="any" value="0" aria-label="Seek" disabled>
          <output class="time" aria-live="off">0:00.00 / 0:00.00</output>
          <button class="volume-toggle" type="button" aria-label="Volume" title="Volume" popovertarget="volume-popup" aria-controls="volume-popup" aria-expanded="false" disabled>${speakerIcon}</button>
          <button class="timeline-toggle" type="button" aria-label="Show timeline" title="Show timeline" aria-expanded="false" aria-controls="timeline-panel" disabled>
            <svg viewBox="0 0 20 20" aria-hidden="true"><path d="m6 8 4 4 4-4" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/></svg>
          </button>
        </div>
        <div class="timeline" id="timeline-panel" part="timeline" hidden></div>
        <div class="volume-popup" id="volume-popup" popover="auto" role="group" aria-label="Volume controls">
          <button class="mute" type="button" aria-label="Mute" title="Mute" aria-pressed="false">${speakerIcon}</button>
          <input class="volume-slider" type="range" min="0" max="1" step="0.01" value="1" aria-label="Volume" autofocus>
          <span class="volume-value">100%</span>
        </div>`;
    this._button = this.shadowRoot.querySelector("button");
    this._slider = this.shadowRoot.querySelector("input");
    this._output = this.shadowRoot.querySelector("output");
    this._timelineToggle = this.shadowRoot.querySelector(".timeline-toggle");
    this._volumeToggle = this.shadowRoot.querySelector(".volume-toggle");
    this._volumePopup = this.shadowRoot.querySelector(".volume-popup");
    this._volumeSlider = this.shadowRoot.querySelector(".volume-slider");
    this._muteButton = this.shadowRoot.querySelector(".mute");
    this._placeVolume = () => {
      const rect = this._volumeToggle.getBoundingClientRect();
      this._volumePopup.style.left = `${Math.max(8, Math.min(window.innerWidth - 216, rect.right - 208))}px`;
      this._volumePopup.style.top = `${rect.top >= 68 ? rect.top - 64 : rect.bottom + 8}px`;
    };
    this._volumePopup.addEventListener("beforetoggle", (event) => {
      const open = event.newState === "open";
      this._volumeToggle.setAttribute("aria-expanded", String(open));
      if (open) {
        this._placeVolume();
        window.addEventListener("resize", this._placeVolume);
        window.addEventListener("scroll", this._placeVolume, true);
      } else {
        window.removeEventListener("resize", this._placeVolume);
        window.removeEventListener("scroll", this._placeVolume, true);
      }
    });
    this._volumeSlider.addEventListener("input", () => {
      if (!this._target) return;
      this._target.volume = Number(this._volumeSlider.value);
      this._target.muted = false;
    });
    this._muteButton.addEventListener("click", () => {
      if (!this._target) return;
      if (this._target.muted || this._target.volume === 0) {
        if (this._target.volume === 0) this._target.volume = 1;
        this._target.muted = false;
      } else this._target.muted = true;
    });
    this._timelinePanel = new TimelinePanel(this.shadowRoot.querySelector(".timeline"), (seconds) => this._target?.seek(seconds));
    this._target = null;
    this._assignedTarget = undefined;
    this._observer = null;
    this._onStateChange = () => this._updateControls();
    this._button.addEventListener("click", () => {
      if (this._target.paused) this._target.play();
      else this._target.pause();
    });
    this._slider.addEventListener("input", () => this._target?.seek(Number(this._slider.value)));
    this._timelineToggle.addEventListener("click", () => { this.timelineExpanded = !this.timelineExpanded; });
  }

  connectedCallback() {
    // Preserve property assignments made before the element was upgraded.
    for (const property of ["target", "timelineExpanded"]) {
      if (Object.hasOwn(this, property)) {
        const value = this[property];
        delete this[property];
        this[property] = value;
      }
    }
    this._watchTarget();
    this._timelinePanel.connect();
    this._updateExpansion();
  }

  disconnectedCallback() {
    if (this._volumePopup.matches(":popover-open")) this._volumePopup.hidePopover();
    window.removeEventListener("resize", this._placeVolume);
    window.removeEventListener("scroll", this._placeVolume, true);
    this._volumeToggle.setAttribute("aria-expanded", "false");
    this._observer?.disconnect();
    this._observer = null;
    this._timelinePanel.disconnect();
    this._bind(null);
  }

  attributeChangedCallback(name) {
    if (name === "timeline-expanded") {
      this._updateExpansion();
      return;
    }
    this._assignedTarget = undefined;
    this._watchTarget();
  }

  get target() { return this._assignedTarget !== undefined ? this._assignedTarget : this._target; }
  set target(target) {
    this._assignedTarget = target;
    this._watchTarget();
  }

  get timelineExpanded() { return this.hasAttribute("timeline-expanded"); }
  set timelineExpanded(value) { this.toggleAttribute("timeline-expanded", Boolean(value)); }

  _updateExpansion() {
    const open = this.timelineExpanded;
    this._timelineToggle.setAttribute("aria-expanded", String(open));
    this._timelineToggle.setAttribute("aria-label", open ? "Hide timeline" : "Show timeline");
    this._timelineToggle.title = open ? "Hide timeline" : "Show timeline";
    this._timelinePanel.setVisible(open);
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
      if (this._volumePopup.matches(":popover-open")) this._volumePopup.hidePopover();
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
    const hasAudio = target?.ready && target.hasAudio;
    const volume = target?.volume ?? 1;
    const silent = target?.muted || volume === 0;
    this._volumeToggle.disabled = !hasAudio;
    this._volumeSlider.disabled = this._muteButton.disabled = !hasAudio;
    if (!hasAudio && this._volumePopup.matches(":popover-open")) this._volumePopup.hidePopover();
    this._volumeSlider.value = String(volume);
    this._volumeSlider.style.setProperty("--progress", `${volume * 100}%`);
    this._volumeSlider.setAttribute("aria-valuetext", `${Math.round(volume * 100)} percent`);
    this.shadowRoot.querySelector(".volume-value").textContent = `${Math.round(volume * 100)}%`;
    for (const button of [this._volumeToggle, this._muteButton]) button.toggleAttribute("data-silent", silent);
    const volumeLabel = hasAudio ? `Volume: ${silent ? "muted" : `${Math.round(volume * 100)}%`}` : "No audio";
    this._volumeToggle.setAttribute("aria-label", volumeLabel);
    this._volumeToggle.title = volumeLabel;
    this._muteButton.setAttribute("aria-pressed", String(Boolean(silent)));
    this._muteButton.setAttribute("aria-label", silent ? "Unmute" : "Mute");
    this._muteButton.title = silent ? "Unmute" : "Mute";
    const timeline = target?.ready ? target.timeline : null;
    this._timelineToggle.disabled = !timeline?.roots.length;
    this._timelinePanel.setTimeline(timeline);
    this._timelinePanel.setTime(time);
  }
}

customElements.define("ranim-controls", RanimControls);

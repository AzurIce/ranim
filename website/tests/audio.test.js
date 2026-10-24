// Real WASM, AudioContext and device clock; click Run to grant user activation.
import "../static/ranim-viewport.js";
import "../static/ranim-controls.js";

const results = document.querySelector("#results");
const fixtures = document.querySelector("#fixtures");
const contexts = [];
const NativeAudioContext = window.AudioContext;
window.AudioContext = class extends NativeAudioContext {
  constructor(options) { super(options); this.voices = []; contexts.push(this); }
  createBufferSource() {
    const voice = super.createBufferSource();
    const analyser = this.createAnalyser();
    voice.connect(analyser);
    voice.analyser = analyser;
    const start = voice.start.bind(voice), stop = voice.stop.bind(voice);
    voice.start = (...args) => { voice.startArgs = args; return start(...args); };
    voice.stop = () => { voice.stopped = true; return stop(); };
    this.voices.push(voice);
    return voice;
  }
  createGain() {
    const gain = super.createGain(), analyser = this.createAnalyser();
    gain.connect(analyser);
    this.output = { analyser, gain };
    return gain;
  }
};
function assert(ok, message) { if (!ok) throw new Error(message); }
const frame = () => new Promise(requestAnimationFrame);
async function eventually(predicate) {
  const deadline = performance.now() + 15000;
  while (!predicate()) { assert(performance.now() < deadline, "Timed out"); await frame(); }
}
function signal(voice) {
  const samples = new Float32Array(voice.analyser.fftSize);
  voice.analyser.getFloatTimeDomainData(samples);
  return samples.some((value) => Math.abs(value) > 0.001);
}
async function viewport(example, scene = example) {
  const view = document.createElement("ranim-viewport");
  view.setAttribute("src", `../static/examples/${example}/pkg/ranim_example.js`);
  view.setAttribute("scene", scene);
  fixtures.append(view);
  await eventually(() => view.ready || view.shadowRoot.querySelector(".status").textContent.startsWith("Unable"));
  assert(view.ready, view.shadowRoot.querySelector(".status").textContent);
  return view;
}
document.querySelector("#start").addEventListener("click", async (event) => {
  event.target.disabled = true;
  const passed = [];
  try {
    const { default: init, app } = await import("../static/examples/midi_visualizer/pkg/ranim_example.js");
    await init();
    const project = app(), canvas = document.createElement("canvas");
    canvas.width = 320; canvas.height = 180; fixtures.append(canvas);
    const raw = await project.createPreview(canvas, "midi_visualizer");
    project.free();
    const mix = raw.audio(), duration = raw.duration;
    assert(mix.sampleRate === 48000 && mix.channels.length === 2, "Incorrect master PCM format");
    assert(mix.channels.every((c) => c instanceof Float32Array && Math.abs(c.length / mix.sampleRate - duration) < 1 / mix.sampleRate), "Audio length differs from scene duration");
    assert(mix.channels.some((c) => c.some((x) => Math.abs(x) > 0.01)), "MIDI mix is silent");
    raw.dispose();
    let disposed = false;
    try { raw.audio(); } catch (error) { disposed = String(error).includes("disposed"); }
    raw.free(); canvas.remove();
    assert(disposed && mix.channels[0].length > 0, "Audio snapshot lifetime is incorrect");
    passed.push("WASM MIDI PCM / stereo format / duration / snapshot ownership / disposal");

    const view = await viewport("midi_visualizer");
    const controls = document.createElement("ranim-controls"), other = document.createElement("ranim-controls");
    controls.target = other.target = view; fixtures.append(controls, other);
    assert(contexts.length === 0, "Scene loading opened an audio device");
    view.seek(0.2);
    assert(contexts.length === 0 && view.paused, "Paused seek started audio");
    controls.shadowRoot.querySelector("button").click();
    const context = contexts[0], first = context.voices[0];
    await eventually(() => context.state === "running" && view.currentTime > 0.3 && signal(first));
    assert(first.startArgs[1] === 0.2, "Audio did not start from the visual seek position");
    for (let c = 0; c < 2; c++) {
      const actual = first.buffer.getChannelData(c);
      for (let i = 0; i < actual.length; i += 997) assert(actual[i] === mix.channels[c][i], "Web Audio changed the PCM channel mapping");
    }
    // Suspending the real audio clock must also stop visual time, even though
    // requestAnimationFrame and wall time keep running.
    await context.suspend();
    const frozen = view.currentTime;
    for (let i = 0; i < 6; i++) await frame();
    assert(view.currentTime === frozen, "Visual ran ahead of suspended audio");
    await context.resume();
    await eventually(() => view.currentTime > frozen + 0.05);
    const voiceCount = context.voices.length;
    controls.shadowRoot.querySelector(".volume-toggle").click();
    assert(controls.shadowRoot.querySelector(".volume-popup").matches(":popover-open"), "Volume button did not open its panel");
    const volumeSlider = controls.shadowRoot.querySelector(".volume-slider");
    volumeSlider.value = "0.35"; volumeSlider.dispatchEvent(new Event("input"));
    assert(view.volume === 0.35 && Number(other.shadowRoot.querySelector(".volume-slider").value) === 0.35, "Control bars did not share volume");
    await eventually(() => Math.abs(context.output.gain.gain.value - 0.35) < 0.005);
    controls.shadowRoot.querySelector(".mute").click();
    assert(view.muted && view.volume === 0.35 && other.shadowRoot.querySelector(".mute").getAttribute("aria-pressed") === "true", "Mute lost volume or failed to synchronize");
    await eventually(() => !signal(context.output));
    assert(context.voices.length === voiceCount && !first.stopped && !view.paused, "Volume change restarted playback");
    controls.shadowRoot.querySelector(".mute").click();
    await eventually(() => signal(context.output));
    assert(!view.muted && view.volume === 0.35, "Unmute lost the previous volume");
    controls.shadowRoot.querySelector(".volume-popup").hidePopover();
    other.shadowRoot.querySelector("button").click();
    const pausedAt = view.currentTime;
    for (let i = 0; i < 4; i++) await frame();
    assert(view.paused && first.stopped && view.currentTime === pausedAt, "Pause did not freeze sound and picture");
    controls.timelineExpanded = true;
    const selected = controls.shadowRoot.querySelector("[part=timeline]").shadowRoot.querySelector(".clip");
    selected.click();
    assert(view.currentTime === pausedAt, "Selecting a clip sought the paused player");
    selected.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
    assert(view.currentTime === pausedAt, "Clip keyboard selection sought the player");
    const voicesAtPause = context.voices.length;
    view.seek(2);
    assert(view.currentTime === 2 && context.voices.length === voicesAtPause, "Paused seek played a voice");
    view.play();
    await eventually(() => view.currentTime > 2.05);
    const beforeSeek = context.voices.at(-1);
    const slider = controls.shadowRoot.querySelector("input");
    slider.value = "4"; slider.dispatchEvent(new Event("input"));
    assert(beforeSeek.stopped && context.voices.at(-1).startArgs[1] === 4, "Playing slider seek did not replace the audio offset");
    assert(Number(other.shadowRoot.querySelector("input").value) >= 4, "Second controls lost seek state");
    await eventually(() => view.currentTime > 4.05 && signal(context.voices.at(-1)));
    controls.timelineExpanded = true;
    const panel = controls.shadowRoot.querySelector("[part=timeline]").shadowRoot;
    panel.querySelector("[role=slider]").dispatchEvent(new KeyboardEvent("keydown", { key: "Home", bubbles: true }));
    assert(context.voices.at(-1).startArgs[1] === 0 && !view.paused, "Timeline seek did not restart audio");
    assert(contexts.length === 1, "Seeking or extra controls opened more audio devices");
    passed.push("audible MIDI output / device-clock synchronization / play / pause / seek / multiple controls / timeline seeking / volume / mute");

    view.seek(view.duration - 0.06);
    await eventually(() => view.paused);
    assert(view.currentTime === view.duration && context.voices.at(-1).stopped, "Scene end did not stop both planes");
    view.play();
    assert(context.voices.at(-1).startArgs[1] === 0, "Replay did not restart the soundtrack");
    view.remove();
    await eventually(() => context.state === "closed");
    assert(context.voices.at(-1).stopped && !view.ready, "Unmount retained an audible voice");
    fixtures.append(view); await eventually(() => view.ready);
    assert(view.currentTime === 0 && view.paused && contexts.length === 1, "Reconnect reused playback state");
    view.play(); await eventually(() => contexts[1]?.state === "running");
    const replacedContext = contexts[1];
    view.setAttribute("scene", "midi_visualizer_per_note");
    await eventually(() => view.ready && replacedContext.state === "closed");
    assert(view.paused && view.currentTime === 0, "Scene switch retained its soundtrack clock");
    assert(view.volume === 0.35 && !view.muted, "Scene switch lost volume settings");
    view.play(); await eventually(() => Math.abs(contexts.at(-1).output?.gain.gain.value - 0.35) < 0.005);
    view.remove();
    const silent = await viewport("arc");
    const silentProject = (await import("../static/examples/arc/pkg/ranim_example.js")).app();
    const silentCanvas = document.createElement("canvas");
    const silentRaw = await silentProject.createPreview(silentCanvas, "arc"); silentProject.free();
    assert(silentRaw.audio() === null, "Silent scene exported an audio buffer"); silentRaw.dispose(); silentRaw.free();
    const devices = contexts.length;
    silent.play(); await eventually(() => silent.currentTime > 0.05);
    assert(contexts.length === devices, "Silent playback opened an audio device"); silent.remove();
    const demo = await viewport("audio_demo");
    let audioErrors = 0;
    demo.addEventListener("error", () => ++audioErrors);
    const WorkingContext = window.AudioContext;
    window.AudioContext = class { constructor() { throw new Error("Test output device failure"); } };
    demo.play();
    assert(demo.paused && demo.ready && audioErrors === 1, "Device failure discarded the scene or left its clock running");
    window.AudioContext = WorkingContext;
    demo.play();
    await eventually(() => contexts.at(-1).state === "running" && signal(contexts.at(-1).voices.at(-1)));
    demo.pause();
    const demoContext = contexts.at(-1), resume = demoContext.resume.bind(demoContext);
    demoContext.resume = () => Promise.reject(new Error("Test resume rejection"));
    demo.play(); await eventually(() => audioErrors === 2);
    assert(demo.ready && demo.paused && demoContext.voices.at(-1).stopped, "Rejected resume left an active voice");
    demoContext.resume = resume;
    demo.play(); await eventually(() => signal(demoContext.voices.at(-1)));
    demo.remove();
    passed.push("end / replay / unmount / reconnect / scene replacement / silent clock / synthesized audio demo / device and resume failure recovery");
    results.dataset.status = "passed";
  } catch (error) {
    results.dataset.status = "failed";
    passed.push(`FAIL ${error?.stack ?? error}`);
  } finally {
    fixtures.replaceChildren();
    window.AudioContext = NativeAudioContext;
    results.textContent = passed.map((name) => name.startsWith("FAIL") ? name : `PASS ${name}`).join("\n");
  }
}, { once: true });

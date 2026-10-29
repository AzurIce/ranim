// Serve the repository root after `cargo examples build arc`, then open
// /website/tests/preview.test.html. These use a real WebGPU device and WASM module.
const results = document.getElementById("results");
const fixtures = document.getElementById("fixtures");
const src = "../static/examples/arc/pkg/ranim_example.js";
const frame = () => new Promise(requestAnimationFrame);

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function expectEvent(target, name, action) {
  return new Promise((resolve, reject) => {
    const done = (error) => {
      clearTimeout(timer);
      target.removeEventListener(name, success);
      target.removeEventListener("error", failure);
      error ? reject(error) : resolve();
    };
    const success = () => done();
    const failure = (event) => done(event.detail);
    const timer = setTimeout(() => done(new Error(`Timed out waiting for ${name}`)), 15000);
    target.addEventListener(name, success);
    target.addEventListener("error", failure);
    try { action(); } catch (error) { done(error); }
  });
}

async function eventually(predicate) {
  const deadline = performance.now() + 5000;
  while (!predicate()) {
    assert(performance.now() < deadline, "Timed out waiting for state update");
    await frame();
  }
}

const button = (controls) => controls.shadowRoot.querySelector("button");
const slider = (controls) => controls.shadowRoot.querySelector("input");
const timeline = (controls) => controls.shadowRoot.querySelector("[part=timeline]").shadowRoot;

function newViewport(id) {
  const viewport = document.createElement("ranim-viewport");
  viewport.id = id;
  viewport.setAttribute("src", src);
  viewport.setAttribute("scene", "arc");
  return viewport;
}

const passed = [];
try {
  // A WASM bin runs its empty main during initialization. It must remain
  // available for explicit app() calls and defer GPU work until createPreview.
  const requestAdapter = navigator.gpu.requestAdapter;
  let gpuRequests = 0;
  navigator.gpu.requestAdapter = function (...args) {
    gpuRequests++;
    return requestAdapter.apply(this, args);
  };
  try {
    const { default: init, app } = await import(src);
    await init();
    for (let i = 0; i < 2; i++) {
      const project = app();
      assert(project.sceneNames() === "arc", "Empty main left the module unusable");
      project.free();
    }
    assert(gpuRequests === 0 && fixtures.childElementCount === 0, "Module initialization started a preview");
  } finally {
    navigator.gpu.requestAdapter = requestAdapter;
  }
  passed.push("WASM bin initialization / explicit app creation / deferred GPU setup");

  // Property binding before module registration must survive element upgrade.
  const primary = document.createElement("ranim-viewport");
  primary.id = "primary-preview";
  const direct = document.createElement("ranim-controls");
  direct.target = primary;
  fixtures.append(direct);
  await import("../static/ranim-controls.js");
  assert(!customElements.get("ranim-viewport"), "Controls imported the viewport implementation");
  assert(!direct.shadowRoot.querySelector("canvas"), "Controls own a rendering canvas");
  assert(button(direct).disabled, "Unready target enabled controls");

  // The ID-bound controls precede their viewport in a different container.
  const toolbar = document.createElement("aside");
  const controls = document.createElement("ranim-controls");
  controls.setAttribute("for", "primary-preview");
  toolbar.append(controls);
  fixtures.append(toolbar);
  assert(controls.target === null && button(controls).disabled, "Missing target should disable controls");
  await import("../static/ranim-viewport.js");
  primary.setAttribute("src", src);
  primary.setAttribute("scene", "arc");
  await expectEvent(primary, "ready", () => fixtures.append(primary));
  await eventually(() => controls.target === primary && !button(controls).disabled);
  assert(direct.target === primary && !button(direct).disabled, "Pre-upgrade property target was lost");
  assert(!primary.shadowRoot.querySelector("button"), "Viewport contains playback controls");
  assert(primary.ready && primary.duration === 3 && primary.paused, "Expected a paused three-second scene");

  assert(primary.timeline.duration === 3 && primary.timeline.roots.length > 0, "Viewport did not expose animation metadata");
  const { projectTimeline } = await import("../static/ranim-timeline.js");
  const projected = projectTimeline(primary.timeline);
  const lagged = projected.find((node) => node.children.length === 100);
  assert(lagged && Math.abs(lagged.children.at(-1).end - 3) < 1e-6, "WASM content timing was not projected into scene seconds");
  assert(!controls.timelineExpanded, "Timeline should start collapsed");
  const originalCanvas = primary.shadowRoot.querySelector("canvas");
  controls.shadowRoot.querySelector(".timeline-toggle").click();
  assert(controls.timelineExpanded && controls.target === primary, "Opening timeline changed target binding");
  const rowsBefore = timeline(controls).querySelectorAll(".row").length;
  assert(rowsBefore >= 100, "Animation tree did not disclose its parallel tracks");
  primary.seek(1.5);
  assert(timeline(controls).querySelector("[role=slider]").getAttribute("aria-valuenow") === "1.5", "Timeline playhead missed seek state");
  const clip = timeline(controls).querySelector(".clip");
  clip.click();
  assert(primary.currentTime === 1.5, "Selecting a clip changed the playhead");
  assert(clip.getAttribute("aria-pressed") === "true", "Clip selection did not update inspection state");
  assert(timeline(controls).querySelector(".selection-range").textContent.includes("—"), "Selected clip has no timing inspector");
  timeline(controls).querySelector(".tree-toggle").click();
  const collapsedRows = timeline(controls).querySelectorAll(".row").length;
  assert(collapsedRows < rowsBefore, "Collapsing animation group did not hide tracks");
  controls.timelineExpanded = false;
  controls.timelineExpanded = true;
  assert(timeline(controls).querySelectorAll(".row").length === collapsedRows, "Closing timeline lost its tree expansion state");
  timeline(controls).querySelector(".zoom-in").click();
  assert(timeline(controls).querySelector(".zoom-level").textContent === "200%", "Timeline zoom failed");
  timeline(controls).querySelector(".fit").click();
  assert(timeline(controls).querySelector(".zoom-level").textContent === "100%", "Fit did not reset timeline zoom");
  timeline(controls).querySelector("[role=slider]").dispatchEvent(new KeyboardEvent("keydown", { key: "End", bubbles: true }));
  assert(primary.currentTime === 3, "Timeline keyboard seek failed");
  direct.timelineExpanded = true;
  primary.seek(1.25);
  assert(timeline(direct).querySelector("[role=slider]").getAttribute("aria-valuenow") === "1.25", "Second timeline did not synchronize");
  assert(timeline(direct).querySelectorAll(".row").length === rowsBefore, "Control bars shared tree expansion state");
  assert(primary.shadowRoot.querySelector("canvas") === originalCanvas && primary.ready, "Expanding controls recreated the renderer");
  controls.timelineExpanded = direct.timelineExpanded = false;
  primary.seek(0);
  passed.push("WASM tree metadata / projected timing / expandable tracks / selection / zoom / synchronized timelines");

  await expectEvent(primary, "timeupdate", () => {
    slider(controls).value = "1.5";
    slider(controls).dispatchEvent(new Event("input"));
  });
  assert(primary.currentTime === 1.5 && Number(slider(direct).value) === 1.5, "Controls did not share seek state");
  button(controls).click();
  await eventually(() => primary.currentTime > 1.55);
  assert(button(direct).getAttribute("aria-label") === "Pause", "Second toolbar missed play event");
  button(direct).click();
  const pausedAt = primary.currentTime;
  for (let i = 0; i < 4; i++) await frame();
  assert(primary.currentTime === pausedAt, "Paused clock kept advancing");
  primary.seek(0.2);
  assert(primary.currentTime === 0.2 && Number(slider(controls).value) === 0.2, "Backward seek failed");
  primary.seek(2.99);
  await expectEvent(primary, "ended", () => primary.play());
  assert(primary.paused && primary.currentTime === 3, "End of playback did not pause");
  assert(button(controls).getAttribute("aria-label") === "Play", "Toolbar missed playback end");
  primary.play();
  assert(primary.currentTime < 1, "Playing at the end did not restart");
  primary.pause();
  passed.push("independent modules / late ID target / pre-upgrade property / multiple controls / playback");

  const canvas = primary.shadowRoot.querySelector("canvas");
  const savedTime = primary.currentTime;
  controls.remove();
  direct.remove();
  assert(direct.target === primary && button(direct).disabled, "Detached controls lost their configured target or kept active listeners");
  primary.play();
  await eventually(() => primary.currentTime > savedTime + 0.05);
  primary.pause();
  assert(primary.ready && primary.shadowRoot.querySelector("canvas") === canvas, "Removing controls recreated the session");
  fixtures.append(direct);
  toolbar.append(controls);
  assert(controls.target === primary, "Reconnected toolbar lost its target");
  assert(Math.abs(Number(slider(controls).value) - primary.currentTime) < 1e-6, "Reconnected toolbar missed current state");
  passed.push("controls can be moved / removed / reconnected without disposing the viewport");

  const second = newViewport("secondary-preview");
  await expectEvent(second, "ready", () => fixtures.append(second));
  primary.seek(1);
  second.seek(2);
  assert(primary.currentTime === 1 && second.currentTime === 2, "Canvas sessions share a clock");
  controls.target = second;
  assert(Number(slider(controls).value) === 2, "Property rebinding missed target state");
  primary.seek(0.5);
  assert(Number(slider(controls).value) === 2, "Toolbar still listens to its old target");
  button(controls).click();
  assert(!second.paused && primary.paused, "Toolbar controlled the wrong viewport");
  second.pause();
  controls.setAttribute("for", "primary-preview");
  assert(controls.target === primary, "Changing for did not replace explicit target");
  controls.target = null;
  assert(controls.target === null && button(controls).disabled, "Clearing target did not detach controls");
  controls.setAttribute("for", "secondary-preview");
  assert(controls.target === second, "ID rebinding failed");

  let updates = 0;
  second.addEventListener("timeupdate", () => ++updates);
  second.play();
  second.remove();
  for (let i = 0; i < 4; i++) await frame();
  assert(updates === 0 && second.paused && !second.ready, "Removed viewport still schedules frames");
  assert(controls.target === null && button(controls).disabled, "Toolbar retained a removed ID target");
  await expectEvent(second, "ready", () => fixtures.append(second));
  await eventually(() => controls.target === second && !button(controls).disabled);
  assert(second.currentTime === 0, "Reconnected viewport reused a disposed session");

  // Replacing an element with the same ID must bind the new instance.
  second.remove();
  const replacement = newViewport("secondary-preview");
  await expectEvent(replacement, "ready", () => fixtures.append(replacement));
  await eventually(() => controls.target === replacement);
  assert(!button(controls).disabled, "Replacement target did not enable controls");
  replacement.id = "renamed-preview";
  await eventually(() => controls.target === null);
  controls.setAttribute("for", "renamed-preview");
  assert(controls.target === replacement, "ID changes were not observed");
  passed.push("independent canvases / target switching / ID changes / replacement / viewport reconnection");

  controls.timelineExpanded = true;
  await expectEvent(replacement, "error", () => replacement.setAttribute("scene", "not-a-scene"));
  assert(button(controls).disabled && !replacement.ready, "Failed preview left controls enabled");
  assert(replacement.timeline === null && !timeline(controls).querySelector(".row"), "Failed preview retained the old animation tree");
  await expectEvent(replacement, "ready", () => replacement.setAttribute("scene", "arc"));
  assert(!button(controls).disabled, "Recovered preview did not enable controls");
  assert(timeline(controls).querySelector(".row"), "Recovered preview did not restore its animation tree");
  replacement.removeAttribute("src");
  assert(!replacement.ready && button(controls).disabled, "Clearing source did not clear controls");
  passed.push("load failure / recovery / source removal reset bound controls");

  // ID lookup is scoped to the controls' root, while property binding crosses roots.
  const host = document.createElement("div");
  const root = host.attachShadow({ mode: "open" });
  const inner = newViewport("primary-preview");
  const innerControls = document.createElement("ranim-controls");
  innerControls.setAttribute("for", "primary-preview");
  root.append(innerControls, inner);
  await expectEvent(inner, "ready", () => fixtures.append(host));
  await eventually(() => innerControls.target === inner);
  assert(innerControls.target !== primary, "ID lookup escaped its shadow root");
  controls.target = inner;
  inner.seek(1.2);
  assert(Number(slider(controls).value) === 1.2, "Property binding did not cross shadow roots");
  host.remove();
  assert(button(controls).disabled, "Detached shadow viewport left external controls enabled");
  passed.push("shadow-root scoped IDs / explicit binding across roots");

  const { app } = await import(src);
  const project = app();
  const rawCanvas = document.createElement("canvas");
  rawCanvas.width = 320;
  rawCanvas.height = 180;
  fixtures.append(rawCanvas);
  const names = project.sceneNames();
  let invalidScene = false;
  try { await project.createPreview(rawCanvas, "not-a-scene"); }
  catch (error) { invalidScene = String(error).includes("not registered"); }
  assert(invalidScene && project.sceneNames() === names, "Invalid selection consumed the project");
  const session = await project.createPreview(rawCanvas, "arc");
  project.free();
  try {
    let invalidTime = false;
    try { session.renderAt(NaN); }
    catch (error) { invalidTime = String(error).includes("finite"); }
    assert(invalidTime, "Non-finite time was accepted");
    for (let i = 0; !session.renderAt(1.5); i++) {
      assert(i < 10, "Canvas surface stayed unavailable");
      await frame();
    }
    session.resize(480, 270);
    session.renderAt(2);
    assert(rawCanvas.width === 480 && rawCanvas.height === 270, "Resize did not update the canvas");
    const snapshot = session.timeline();
    assert(snapshot.duration === 3 && snapshot.roots.length > 0, "WASM timeline snapshot is empty");
    session.dispose();
    session.dispose();
    assert(snapshot.roots.length > 0, "Disposing renderer invalidated its metadata snapshot");
    let disposed = false;
    try { session.renderAt(0); }
    catch (error) { disposed = String(error).includes("disposed"); }
    assert(disposed, "Disposed session still renders");
    disposed = false;
    try { session.timeline(); }
    catch (error) { disposed = String(error).includes("disposed"); }
    assert(disposed, "Disposed session still exposes a live timeline getter");
  } finally {
    session.free();
    rawCanvas.remove();
  }
  passed.push("WASM scene selection / validation / rendering / resize / disposal");
  results.textContent = passed.map((name) => `PASS ${name}`).join("\n");
  results.dataset.status = "passed";
} catch (error) {
  results.textContent = `${passed.map((name) => `PASS ${name}\n`).join("")}FAIL ${error?.stack ?? error}`;
  results.dataset.status = "failed";
} finally {
  fixtures.replaceChildren();
}

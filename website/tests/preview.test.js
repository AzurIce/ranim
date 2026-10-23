// Serve the repository root after `cargo examples build arc`, then open
// /website/tests/preview.test.html. These use a real WebGPU device and WASM module.
import "../static/ranim-player.js";

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
    assert(performance.now() < deadline, "Timed out waiting for playback");
    await frame();
  }
}

const passed = [];
try {
  const player = document.createElement("ranim-player");
  player.setAttribute("src", src);
  player.setAttribute("scene", "arc");
  await expectEvent(player, "ready", () => fixtures.append(player));
  assert(player.duration === 3 && player.paused, "Expected a paused three-second scene");
  await expectEvent(player, "timeupdate", () => player.seek(1.5));
  assert(player.currentTime === 1.5, "Seek did not set the playhead");
  player.play();
  await eventually(() => player.currentTime > 1.55);
  player.pause();
  const pausedAt = player.currentTime;
  for (let i = 0; i < 4; i++) await frame();
  assert(player.currentTime === pausedAt, "Paused clock kept advancing");
  await expectEvent(player, "timeupdate", () => player.seek(0.2));
  assert(player.currentTime === 0.2, "Backward seek failed");
  player.seek(2.99);
  await expectEvent(player, "ended", () => player.play());
  assert(player.paused && player.currentTime === 3, "End of playback did not pause");
  player.play();
  assert(player.currentTime < 1, "Playing at the end did not restart");
  player.pause();
  passed.push("play / pause / seek / backward seek / end / replay");

  const second = document.createElement("ranim-player");
  second.setAttribute("src", src);
  second.setAttribute("scene", "arc");
  await expectEvent(second, "ready", () => fixtures.append(second));
  player.seek(1);
  second.seek(2);
  assert(player.currentTime === 1 && second.currentTime === 2, "Canvas sessions share a clock");
  let updates = 0;
  second.addEventListener("timeupdate", () => ++updates);
  second.play();
  second.remove();
  for (let i = 0; i < 4; i++) await frame();
  assert(updates === 0 && second.paused, "Removed player still schedules frames");
  await expectEvent(second, "ready", () => fixtures.append(second));
  assert(second.currentTime === 0, "Reconnected player reused a disposed session");
  second.remove();
  passed.push("independent canvases / removal / reconnection");

  const { app } = await import(src);
  const project = app();
  const canvas = document.createElement("canvas");
  canvas.width = 320;
  canvas.height = 180;
  fixtures.append(canvas);
  const names = project.sceneNames();
  let invalidScene = false;
  try { await project.createPreview(canvas, "not-a-scene"); }
  catch (error) { invalidScene = String(error).includes("not registered"); }
  assert(invalidScene && project.sceneNames() === names, "Invalid selection consumed the project");
  const session = await project.createPreview(canvas, "arc");
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
    assert(canvas.width === 480 && canvas.height === 270, "Resize did not update the canvas");
    session.dispose();
    session.dispose();
    let disposed = false;
    try { session.renderAt(0); }
    catch (error) { disposed = String(error).includes("disposed"); }
    assert(disposed, "Disposed session still renders");
  } finally {
    session.free();
    canvas.remove();
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

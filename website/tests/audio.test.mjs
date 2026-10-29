import test, { beforeEach } from "node:test";
import assert from "node:assert/strict";
import { AudioPlayback } from "../static/ranim-audio.js";

// Controllable device time lets these checks distinguish an audio clock from
// elapsed wall time, and exercise late browser permission failures precisely.
let contexts;
class Buffer {
  constructor({ sampleRate, numberOfChannels, length }) {
    this.duration = length / sampleRate;
    this.channels = Array.from({ length: numberOfChannels }, () => new Float32Array(length));
  }
  copyToChannel(data, i) { this.channels[i].set(data); }
}
class Context {
  constructor() {
    this.currentTime = 10;
    this.state = "running";
    this.destination = {};
    this.voices = [];
    this.pending = [];
    contexts.push(this);
  }
  createBufferSource() {
    const voice = {
      connect() {}, disconnect() { this.disconnected = true; },
      start(...args) { this.startArgs = args; }, stop() { this.stopped = true; },
    };
    this.voices.push(voice);
    return voice;
  }
  createGain() {
    const gain = { value: 1, setTargetAtTime(value) { this.value = value; } };
    this.output = { gain, connect() {}, disconnect() { this.disconnected = true; } };
    return this.output;
  }
  resume() { return new Promise((resolve, reject) => this.pending.push({ resolve, reject })); }
  getOutputTimestamp() { return this.outputTime === undefined ? { performanceTime: 0 } : { contextTime: this.outputTime, performanceTime: performance.now() }; }
  async close() { this.state = "closed"; }
}
beforeEach(() => {
  contexts = [];
  globalThis.AudioBuffer = Buffer;
  globalThis.AudioContext = Context;
});
function player() {
  return new AudioPlayback({ sampleRate: 2, channels: [new Float32Array([1, 2, 3, 4]), new Float32Array([5, 6, 7, 8])] }, 2);
}

test("loads distinct PCM channels without opening an output device", () => {
  const p = player();
  assert.equal(contexts.length, 0);
  p.play(0.5);
  const voice = contexts[0].voices[0];
  assert.deepEqual([...voice.buffer.channels[0]], [1, 2, 3, 4]);
  assert.deepEqual([...voice.buffer.channels[1]], [5, 6, 7, 8]);
  assert.deepEqual(voice.startArgs, [10, 0.5, 1.5]);
  p.dispose();
});
test("playhead follows device/output time and never runs before its new voice", () => {
  const p = player(); p.play(0.5);
  const c = contexts[0];
  c.currentTime = 10.8;
  assert.ok(Math.abs(p.position() - 1.3) < 1e-10);
  c.outputTime = 10.6;
  assert.ok(Math.abs(p.position() - 1.1) < 0.001);
  c.outputTime = 9.9;
  assert.equal(p.position(), 0.5);
  c.currentTime = 20; c.outputTime = 20;
  assert.equal(p.position(), 2);
  p.dispose();
});
test("seeking replaces the voice at an absolute offset and reuses the context", () => {
  const p = player(); p.play(0);
  const c = contexts[0], first = c.voices[0];
  c.currentTime = 10.7;
  p.play(1.25);
  assert.ok(first.stopped && first.disconnected);
  assert.equal(contexts.length, 1);
  assert.deepEqual(c.voices[1].startArgs, [10.7, 1.25, 0.75]);
  assert.equal(p.position(), 1.25);
  p.stop(); p.stop();
  assert.ok(c.voices[1].stopped);
  p.play(0.2);
  assert.equal(c.voices.length, 3);
  p.dispose();
  assert.equal(c.state, "closed");
  assert.ok(c.voices[2].stopped);
});
test("old resume rejections cannot stop a seek, pause, or disposed scene", async () => {
  const p = player(); const first = p.play(0);
  const c = contexts[0];
  const second = p.play(1);
  c.pending[0].reject(new Error("old permission request"));
  await first;
  assert.ok(!c.voices[1].stopped);
  p.stop();
  c.pending[1].reject(new Error("paused request"));
  await second;
  const third = p.play(0);
  p.dispose();
  c.pending[2].reject(new Error("removed request"));
  await third;
});
test("current resume rejection stops its voice and reports failure for retry", async () => {
  const p = player(); const started = p.play(0.5);
  const c = contexts[0];
  c.pending[0].reject(new Error("device unavailable"));
  await assert.rejects(started, /device unavailable/);
  assert.ok(c.voices[0].stopped);
  const retry = p.play(0.5);
  c.pending[1].resolve(); await retry;
  assert.ok(!c.voices[1].stopped);
  p.dispose();
});

test("volume changes reuse the active voice and clock, and survive seeking", () => {
  const p = player(); p.setVolume(0.4);
  assert.equal(contexts.length, 0);
  p.play(0.5);
  const c = contexts[0], voice = c.voices[0];
  assert.equal(c.output.gain.value, 0.4);
  c.currentTime = 10.5;
  p.setVolume(0);
  assert.equal(c.output.gain.value, 0);
  assert.equal(p.position(), 1);
  assert.equal(c.voices.length, 1);
  assert.ok(!voice.stopped);
  p.play(0.2);
  assert.equal(c.output.gain.value, 0);
  p.setVolume(0.7);
  assert.equal(c.output.gain.value, 0.7);
  p.dispose();
  assert.ok(c.output.disconnected);
});

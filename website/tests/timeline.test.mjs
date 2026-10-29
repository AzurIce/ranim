import test from "node:test";
import assert from "node:assert/strict";
import { projectTimeline, buildTimelineTracks, defaultExpanded } from "../static/ranim-timeline.js";

function leaf(range, options = {}) {
  return { name: "example::FadeIn<Item>", kind: "anim", range, contentDuration: 1, enabled: true, simStep: null, children: [], ...options };
}
function group(range, contentDuration, children, options = {}) {
  return leaf(range, { name: "ranim_core::AnimSequence", kind: "children", contentDuration, children, ...options });
}
function project(...roots) { return projectTimeline({ duration: 10, roots }); }

test("nested placement maps parent-local content into stretched scene time", () => {
  const data = group([2, 8], 3, [group([0.5, 1], 2, [leaf([1, 2])])]);
  const root = project(data)[0];
  assert.deepEqual([root.children[0].start, root.children[0].end], [3, 4]);
  assert.deepEqual([root.children[0].children[0].start, root.children[0].children[0].end], [3.5, 4]);
  assert.deepEqual(data.children[0].range, [0.5, 1]);
});

test("zero-duration content produces finite point placements", () => {
  const root = project(group([4, 10], 0, [leaf([3, 8])]))[0];
  assert.equal(root.children[0].start, 4);
  assert.equal(root.children[0].end, 4);
});

test("disabled containers mute descendant clips without changing source metadata", () => {
  const source = group([0, 2], 2, [leaf([0, 2])], { enabled: false });
  const root = project(source)[0];
  assert.equal(root.children[0].enabled, false);
  assert.equal(source.children[0].enabled, true);
});

test("nested ordered sequences share one track and preserve gaps", () => {
  const roots = project(group([0, 6], 6, [
    group([0, 4], 4, [leaf([0, 1]), leaf([1, 4])]), leaf([5, 6]),
  ]));
  const rows = buildTimelineTracks(roots, new Set());
  assert.equal(rows.length, 1);
  assert.equal(rows[0].bands.length, 2);
  assert.deepEqual(rows[0].clips.map((node) => [node.start, node.end]), [[0, 1], [1, 4], [5, 6]]);
});

test("parallel children disclose separate rows instead of sharing a track", () => {
  const roots = project(group([0, 3], 3, [leaf([0, 2]), leaf([1, 3])]));
  assert.equal(roots[0].serial, false);
  assert.equal(buildTimelineTracks(roots, new Set()).length, 1);
  const rows = buildTimelineTracks(roots, new Set(["0"]));
  assert.deepEqual(rows.map((row) => row.depth), [0, 1, 1]);
  assert.deepEqual(rows.map((row) => row.node.id), ["0", "0.0", "0.1"]);
});

test("parallel clips embedded in a sequence expand underneath its shared track", () => {
  const roots = project(group([0, 4], 4, [
    group([1, 3], 2, [leaf([0, 2]), leaf([0, 2])]), leaf([3, 4]),
  ]));
  assert.equal(buildTimelineTracks(roots, new Set()).length, 1);
  const rows = buildTimelineTracks(roots, new Set(["0.0"]));
  assert.deepEqual(rows[0].clips.map((clip) => clip.id), ["0.0", "0.1"]);
  assert.deepEqual(rows.slice(1).map((row) => [row.node.start, row.node.end]), [[1, 3], [1, 3]]);
});

test("default expansion counts overlapping containers, not serial wrappers", () => {
  const inner = group([0, 10], 10, [leaf([0, 10]), leaf([0, 10])]);
  const middle = group([0, 10], 10, [inner, leaf([0, 10])]);
  const outer = group([0, 10], 10, [middle, leaf([0, 10])]);
  const roots = project(group([0, 10], 10, [outer]));
  const expanded = defaultExpanded(roots);
  assert(expanded.has("0.0"));
  assert(expanded.has("0.0.0"));
  assert(!expanded.has("0.0.0.0"));
});

test("audio and point clips retain their kind, timing and inspection metadata", () => {
  const roots = project(leaf([2, 2], { kind: "audio", simStep: 0.25 }));
  const clip = buildTimelineTracks(roots, new Set())[0].clips[0];
  assert.equal(clip.kind, "audio");
  assert.equal(clip.simStep, 0.25);
  assert.equal(clip.start, clip.end);
  assert.equal(clip.label, "FadeIn");
});

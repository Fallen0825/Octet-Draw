// Run: node src/cycletool.check.ts. The Cycle tool: a drawn circle is one arc, a click breaks it into steps, a
// structure on the circle joins it (taking a break's place), a deleted intermediate leaves a break, reagents and
// products keep their steps, and all of it survives a save.
import assert from "node:assert/strict";
import { addStop, breakAt, joinCycles, layoutCycles, removeStop, sideFor, stepAt } from "./cycle.ts";
import { DEFAULT_STYLE } from "./docstyle.ts";
import { addCaption, addCycle, addRing, deleteAtoms, deleteGraphics, EMPTY, fragments } from "./edit.ts";
import { readOct, writeOct } from "./oct.ts";
import type { Box, Mol } from "./render.ts";

// Ink: each atom's position as a box a third of a bond across (px, y down).
const L = 30, ink = (m: Mol) => (atoms: number[]): Box | undefined => {
  const ps = atoms.map((i) => m.atoms[i].location);
  return ps.length ? [Math.min(...ps.map((p) => p[0])) * L - 5, -Math.max(...ps.map((p) => p[1])) * L - 5, Math.max(...ps.map((p) => p[0])) * L + 5, -Math.min(...ps.map((p) => p[1])) * L + 5] : undefined;
};
const arcs = (m: Mol) => layoutCycles(m, L, 1, ink(m)).arrows.filter((a) => a.kind === "curved");
const right = 0, top = Math.PI / 2, left = Math.PI, bottom = -Math.PI / 2;

// A new circle with no nodes is one arc, nearly all the way round.
let [m, id] = addCycle(EMPTY, { centre: [0, 0], radius: 3, clockwise: true, nodes: [], at: [] });
assert.equal(arcs(m).length, 1);

// Clicks break it: two breaks are two steps, and a click on a break joins it up again.
m = addStop(m, id, right, []);
m = addStop(m, id, left, []);
assert.equal(m.cycles![0].nodes.length, 2);
assert.equal(arcs(m).length, 2);
const c = () => m.cycles![0];
assert.equal(breakAt(m, c(), left + 0.02, 0.1), 1);
assert.equal(breakAt(m, c(), top, 0.1), undefined);

// Clockwise from the right-hand break, the bottom is step 0 and the top is step 1.
assert.equal(stepAt(m, c(), bottom), 0);
assert.equal(stepAt(m, c(), top), 1);

// A reagent on the top step stays on it when a break goes in on the bottom step, and moves on one when it goes in
// before it.
[m] = addCaption(m, { text: "H2", x: 0, y: 0, cycle: { cycle: id, step: 1, side: "in" } });
m = addStop(m, id, bottom, []);
assert.equal(m.captions![0].cycle!.step, 2);
m = removeStop(m, id, 1);
assert.equal(m.captions![0].cycle!.step, 1);
assert.equal(c().nodes.length, 2);

// A ring drawn on the circle near a break takes its place; one drawn on the circle elsewhere is a new node; one well
// inside the circle stays out.
m = addRing(m, 6, true, { point: [-3, 0] });
m = addRing(m, 6, true, { point: [0, 3] });
m = addRing(m, 6, true, { point: [0.5, 0.5] });
m = joinCycles(m, fragments(m));
const [ringLeft, ringTop, ringIn] = fragments(m);
assert.equal(c().nodes.length, 3);
assert.ok(c().nodes.some((n) => n.length && n.every((i) => ringLeft.includes(i))));
assert.ok(c().nodes.some((n) => n.length && n.every((i) => ringTop.includes(i))));
assert.ok(!c().nodes.flat().some((i) => ringIn.includes(i)));
assert.equal(c().nodes.filter((n) => !n.length).length, 1);

// joinCycles with moved atoms only takes in what moved.
let n = addRing(m, 6, true, { point: [3, 0.2] });
assert.equal(joinCycles(n, fragments(n), () => false).cycles![0].nodes.length, 3);
n = joinCycles(n, fragments(n), (i) => i >= m.atoms.length);
assert.equal(n.cycles![0].nodes.length, 3); // it lands on the right-hand break

// Deleting an intermediate leaves a break where it was.
const before = c().nodes.length;
m = deleteAtoms(m, new Set(ringTop));
assert.equal(c().nodes.length, before);
assert.equal(c().nodes.filter((x) => !x.length).length, 2);
assert.ok(Math.abs(c().at![c().nodes.findIndex((x, k) => !x.length && Math.abs(c().at![k] - top) < 0.2)] - top) < 0.2);

// Saved and opened again: the breaks, their angles and the reagent's step.
const back = readOct(writeOct(m, DEFAULT_STYLE)).mol, y = back.cycles![0];
assert.equal(y.nodes.length, c().nodes.length);
assert.deepEqual(y.nodes.map((x) => x.length), c().nodes.map((x) => x.length));
y.at!.forEach((a, k) => assert.ok(Math.abs(a - c().at![k]) < 1e-3));
assert.equal(back.captions!.find((x) => x.cycle)?.cycle?.step, 1);

// An empty circle is saved and opened as one.
const empty = readOct(writeOct(addCycle(EMPTY, { centre: [1, 1], radius: 2, clockwise: false, nodes: [], at: [] })[0], DEFAULT_STYLE)).mol;
assert.equal(empty.cycles?.[0]?.nodes.length, 0);

// A side arrow's place: unset is where Octet has always put it, and a drag to its outer end reads back as the same
// place (inside the circle too). The text goes beyond that end.
{
  let s: Mol, sid: number;
  [s, sid] = addCycle(EMPTY, { centre: [0, 0], radius: 3, clockwise: true, nodes: [], at: [] });
  s = addStop(addStop(s, sid, right, []), sid, left, []);
  const lay = (x: Mol) => layoutCycles(x, L, 1, ink(x));
  const side = (x: Mol) => lay(x).arrows.find((a) => a.caption !== undefined)!;
  let cap: number;
  [s, cap] = addCaption(s, { text: "H2", x: 0, y: 0, cycle: { cycle: sid, step: 1, side: "in" } });
  const before = side(s), end: [number, number] = [before.from[0] * L, -before.from[1] * L];
  // Default: 0.18 along the top step from the left, 1.1 bond lengths out.
  const back = sideFor(lay(s), s.captions![0].cycle!, end, L)!;
  assert.ok(Math.abs(back.at - 0.18) < 0.02 && Math.abs(back.reach - 1.1) < 0.02, JSON.stringify(back));
  for (const want of [{ at: 0.5, reach: 2 }, { at: 0.8, reach: -1 }, { at: 0.1, reach: 0.6 }]) {
    const t = { ...s.captions![0].cycle!, ...want }, moved = { ...s, captions: s.captions!.map((c) => (c.id === cap ? { ...c, cycle: t } : c)) };
    const a = side(moved), got = sideFor(lay(moved), t, [a.from[0] * L, -a.from[1] * L], L)!;
    assert.ok(Math.abs(got.at - want.at) < 0.02 && Math.abs(got.reach - want.reach) < 0.02, JSON.stringify({ want, got }));
    // Inside, the text anchor is nearer the middle than the arrow's end.
    const anchor = lay(moved).anchors.get(cap)!, r = Math.hypot(anchor.x, anchor.y), e = Math.hypot(a.from[0] * L, a.from[1] * L);
    assert.ok(want.reach < 0 ? r < e : r > e);
  }
  // Saved and opened again.
  const t = { ...s.captions![0].cycle!, at: 0.4, reach: -0.9 }, kept = readOct(writeOct({ ...s, captions: s.captions!.map((c) => ({ ...c, cycle: t })) }, DEFAULT_STYLE)).mol;
  assert.deepEqual([kept.captions![0].cycle!.at, kept.captions![0].cycle!.reach], [0.4, -0.9]);
}

// Passing through: a reagent's arrow touches the arc and goes out again (its head outside the circle); a reagent and
// a product on the same step that both pass through are one arrow from one to the other. Saved and opened again.
{
  let s: Mol, sid: number;
  [s, sid] = addCycle(EMPTY, { centre: [0, 0], radius: 3, clockwise: true, nodes: [], at: [] });
  s = addStop(addStop(s, sid, right, []), sid, left, []);
  const lay = (x: Mol) => layoutCycles(x, L, 1, ink(x)), r = (q: [number, number]) => Math.hypot(...q);
  let rid: number, pid: number;
  [s, rid] = addCaption(s, { text: "H2", x: 0, y: 0, cycle: { cycle: sid, step: 1, side: "in", through: true } });
  let a = lay(s).arrows.find((x) => x.caption === rid)!;
  assert.ok(r(a.from) > 3.5 && r(a.to) > 3.5, "both ends outside");
  [s, pid] = addCaption(s, { text: "H2O", x: 0, y: 0, cycle: { cycle: sid, step: 1, side: "out", through: true } });
  const sides = lay(s).arrows.filter((x) => x.caption !== undefined || x.captionEnd !== undefined);
  assert.equal(sides.length, 1);
  a = sides[0];
  assert.ok(a.caption === rid && a.captionEnd === pid);
  // Stopping at the cycle again: two arrows, each ending on the arc.
  const stop = { ...s, captions: s.captions!.map((c) => ({ ...c, cycle: { ...c.cycle!, through: false } })) };
  assert.equal(lay(stop).arrows.filter((x) => x.caption !== undefined || x.captionEnd !== undefined).length, 2);
  const kept = readOct(writeOct(s, DEFAULT_STYLE)).mol;
  assert.ok(kept.captions!.every((c) => c.cycle?.through === true));
}

// A reagent alone runs into the arc with no head and no tail; with a product on its step (neither kept apart) the two
// are one arrow; kept apart, two. Where it touches the arc can move, and a tail can be pulled out.
{
  let s: Mol, sid: number, rid: number, pid: number;
  [s, sid] = addCycle(EMPTY, { centre: [0, 0], radius: 3, clockwise: true, nodes: [], at: [] });
  s = addStop(addStop(s, sid, right, []), sid, left, []);
  const lay = (x: Mol) => layoutCycles(x, L, 1, ink(x)), r = (q: [number, number]) => Math.hypot(...q), mine = (x: Mol, id: number) => lay(x).arrows.filter((a) => a.caption === id || a.captionEnd === id);
  [s, rid] = addCaption(s, { text: "ArX", x: 0, y: 0, cycle: { cycle: sid, step: 1, side: "in" } });
  let [a] = mine(s, rid);
  assert.ok(r(a.from) > 3.5 && Math.abs(r(a.to) - 3) < 0.05 && a.end === "none", "a reagent runs into the arc");
  const h = lay(s).handles.get(rid)!;
  assert.ok(h.end && h.meet && !h.tail);
  // meet: it touches the arc further along.
  const met = { ...s, captions: s.captions!.map((c) => ({ ...c, cycle: { ...c.cycle!, meet: 0.6 } })) };
  assert.ok(Math.hypot(lay(met).handles.get(rid)!.meet[0] - h.meet[0], lay(met).handles.get(rid)!.meet[1] - h.meet[1]) > 0.5);
  // A tail where it was put.
  const tailed = { ...s, captions: s.captions!.map((c) => ({ ...c, cycle: { ...c.cycle!, through: true, tail: [0.9, 1.5] as [number, number] } })) };
  [a] = mine(tailed, rid);
  assert.ok(Math.abs(r(a.to) - 4.5) < 0.05 && lay(tailed).handles.get(rid)!.tail);
  [s, pid] = addCaption(s, { text: "MX", x: 0, y: 0, cycle: { cycle: sid, step: 1, side: "out" } });
  assert.equal(mine(s, rid).length, 1);
  assert.equal(mine(s, rid)[0].captionEnd, pid);
  const apart = { ...s, captions: s.captions!.map((c) => (c.id === pid ? { ...c, cycle: { ...c.cycle!, through: false } } : c)) };
  assert.equal(lay(apart).arrows.filter((x) => x.caption !== undefined || x.captionEnd !== undefined).length, 2);
  // A step's name sits half way along its step, just outside the circle (no arrow passes through that step), with no
  // arrow of its own.
  let nid: number;
  [s, nid] = addCaption(s, { text: "oxidative addition", x: 0, y: 0, cycle: { cycle: sid, step: 0, side: "name" } });
  const n = lay(s).anchors.get(nid)!;
  assert.ok(Math.hypot(n.x, n.y) > 3 * L && n.y > 0, JSON.stringify(n)); // the bottom step, outside
  assert.equal(lay(s).arrows.filter((x) => x.caption === nid || x.captionEnd === nid).length, 0);
  // Saved and opened again.
  const kept = readOct(writeOct({ ...apart, captions: [...apart.captions!.map((c) => (c.id === rid ? { ...c, cycle: { ...c.cycle!, meet: 0.3, tail: [0.7, 2] as [number, number] } } : c)), ...s.captions!.filter((c) => c.id === nid)] }, DEFAULT_STYLE)).mol;
  const back = (t: string) => kept.captions!.find((c) => c.text === t)!.cycle!;
  assert.equal(back("MX").through, false);
  assert.deepEqual([back("ArX").meet, back("ArX").tail], [0.3, [0.7, 2]]);
  assert.equal(back("oxidative addition").side, "name");
}

// Text on the circle is an intermediate: it takes a break's place (or makes a node), the arcs stop short of it, and
// deleted it leaves a break. A wider gap leaves more space round it.
{
  let s: Mol, sid: number, tid: number;
  [s, sid] = addCycle(EMPTY, { centre: [0, 0], radius: 3, clockwise: true, nodes: [], at: [] });
  s = addStop(addStop(s, sid, right, []), sid, left, []);
  [s, tid] = addCaption(s, { text: "Pd(OAc)2", x: 0, y: 3 });
  s = joinCycles(s, fragments(s));
  assert.deepEqual(s.captions!.find((c) => c.id === tid)!.cycle, { cycle: sid, step: 2, side: "node" }); // clockwise: right, left, then the top
  assert.equal(s.cycles![0].nodes.length, 3);
  // One near a break takes its place.
  let t2: number;
  [s, t2] = addCaption(s, { text: "LPd", x: -3.1, y: 0.1 });
  s = joinCycles(s, fragments(s), () => false, (c) => c.id === t2);
  assert.equal(s.cycles![0].nodes.length, 3);
  assert.equal(s.captions!.find((c) => c.id === t2)!.cycle!.step, 1);
  // The text's ink: a box two bond lengths wide round it. The arcs stop short of it.
  const box = (x: Mol) => (id: number) => { const c = x.captions!.find((y) => y.id === id)!; return [c.x * L - L, -c.y * L - 8, c.x * L + L, -c.y * L + 4] as Box; };
  const spans = (x: Mol) => layoutCycles(x, L, 1, ink(x), box(x)).spans;
  const into = spans(s).find((p) => p.step === 1)!; // from the left-hand text to the top text
  const wide = spans({ ...s, cycles: s.cycles!.map((c) => ({ ...c, gap: 20 })) }).find((p) => p.step === 1)!;
  assert.ok(wide.to < into.to && wide.from > into.from);
  // Not a break: a click there does not join the circle up.
  assert.equal(breakAt(s, s.cycles![0], top, 0.2), undefined);
  // Deleted, it leaves a break where it was.
  const gone = deleteGraphics(s, new Set([tid]));
  assert.equal(gone.cycles![0].nodes.length, 3);
  assert.equal(breakAt(gone, gone.cycles![0], top, 0.2), 2);
  // The cycle deleted, the text stays as text.
  assert.ok(!deleteGraphics(s, new Set([sid])).captions!.find((c) => c.id === tid)!.cycle);
  // Saved and opened again.
  const kept = readOct(writeOct({ ...s, cycles: s.cycles!.map((c) => ({ ...c, gap: 9 })) }, DEFAULT_STYLE)).mol;
  assert.equal(kept.cycles![0].gap, 9);
  assert.equal(kept.captions!.find((c) => c.text === "Pd(OAc)2")!.cycle!.side, "node");
}

console.log("cycle tool ok");

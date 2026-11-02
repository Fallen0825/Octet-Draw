// Catalytic cycles. A cycle holds its intermediates (sets of atoms) in order around a circle. Its arc arrows,
// and the side arrows for reagents coming in and products going out, are worked out afresh from where the
// intermediates are, so editing an intermediate keeps every arrow fitted to it.
import type { Arrow, Box, Caption, Mol } from "./render.ts";

// scale: the cycle's size against the document style (decision 0007): its arrows, gaps and side arrows follow it.
// width: its arrows' line width in points; unset uses the document's. The scale multiplies it.
// at: each node's angle about the centre (radians, drawing units, y up), for a cycle drawn with the Cycle tool. A node
// with no atoms is a break the user clicked on the circle, or a piece of text (a caption on that node); a node whose
// atoms all go becomes a break. A drawn cycle can have any number of nodes (none is a whole circle).
// gap: the clear space between an arc's ends and an intermediate, in points; unset is GAP_PT.
// sides: structures drawn as a reagent coming in or a product going out (Tobias, 2 November 2026). Each is on the step
// its middle is beside, and its arrow runs between the arc and the structure, wherever the structure is moved.
export type CycleSide = { atoms: number[]; side: "in" | "out" };
export type Cycle = { id: number; centre: [number, number]; radius: number; clockwise: boolean; nodes: number[][]; at?: number[]; colour?: string; order?: number; scale?: number; width?: number; gap?: number; sides?: CycleSide[] };
// A cycle with its structures' atoms renumbered by `to` (undefined: the atom went). A structure with no atoms left
// is no longer a reagent or product.
export function mapSides(c: Cycle, to: (i: number) => number | undefined): Cycle {
  if (!c.sides) return c;
  const sides = c.sides.map((s) => ({ ...s, atoms: s.atoms.map(to).filter((i): i is number => i !== undefined) })).filter((s) => s.atoms.length);
  const { sides: _, ...rest } = c;
  return sides.length ? { ...rest, sides } : rest;
}
// A cycle's caption: its centre label; the reagent coming in, the product going out or the name of a step (node k to
// k + 1); or an intermediate written as text (side "node": step is the node).
// at: where along its step a side arrow's outer end (or a step's name) is (0 at the step's start, 1 at its end);
// reach: how far that end is from the circle, in bond lengths (less than 0 is inside it). Unset, a reagent's end is
// 0.18 along and a product's 0.82, 1.1 bond lengths outside; a step's name is half way along, just outside (inside
// where an arrow passes through its step). Dragging the text sets them.
// through: false keeps a reagent and a product on the same step apart (each ends at the arc); otherwise they are one
// arrow, from the reagent, touching the arc, to the product. true on a reagent or product alone gives its arrow a
// tail: it touches the arc and goes out again, to tail (its at and reach; unset, mirrored about where it touches).
// meet: where along its step the arrow touches the arc.
export type CycleText = { cycle: number; step: number; side: "in" | "out" | "centre" | "node" | "name"; at?: number; reach?: number; through?: boolean; meet?: number; tail?: [number, number] };
export const sideAt = (t: CycleText) => t.at ?? (t.side === "in" ? 0.18 : t.side === "out" ? 0.82 : 0.5);
export const sideReach = (t: CycleText) => t.reach ?? (t.side === "name" ? -0.6 : 1.1);
// A reagent or product (with an arrow); one of those or a step's name (placed along a step).
export const isSide = (t?: CycleText) => t?.side === "in" || t?.side === "out";
export const onStep = (t?: CycleText) => isSide(t) || t?.side === "name";
// A pass-through arrow's ends, unset, are THROUGH bond lengths along the arc either side of where it touches (the
// middle of its step for a reagent and product together; 0.4 along for a reagent alone, 0.6 for a product), so it
// makes the same tight swoop on any size of circle.
const THROUGH = 1.3;

type P = [number, number];
const GAP_PT = 6; // clear space between an arc's ends and an intermediate's ink, in pt

// Derived arrows get ids from here down, so they never meet the drawing's own ids.
const DERIVED = -1_000_000;

export type CycleLayout = {
  // caption and captionEnd: a side arrow's text at its start (a reagent's) and at its end (a product's).
  arrows: (Arrow & { cycle: number; step: number; caption?: number; captionEnd?: number })[];
  // Each reagent's and product's handles (drawing units): its arrow's outer end, where it touches the arc, and its tail.
  handles: Map<number, { end: P; meet: P; tail?: P }>;
  // Each step's arc as drawn: px, y down. It runs from angle a0 + dir * from to a0 + dir * to about centre.
  spans: { cycle: number; step: number; centre: P; R: number; a0: number; dir: number; from: number; to: number }[];
  anchors: Map<number, { x: number; y: number; anchor: "start" | "middle" | "end" }>; // caption id → anchor (px)
};

// k: px per bond length; pt: px per point. ink: an intermediate's ink box in px (y down); text: a caption's.
export function layoutCycles(m: Mol, k: number, pt: number, ink: (atoms: number[]) => Box | undefined, text: (id: number) => Box | undefined = () => undefined): CycleLayout {
  const out: CycleLayout = { arrows: [], anchors: new Map(), spans: [], handles: new Map() };
  let next = DERIVED;
  for (const c of m.cycles ?? []) {
    const f = c.scale ?? 1, own = f !== 1 ? { scale: f } : {}, centre: P = [c.centre[0] * k, -c.centre[1] * k], R = c.radius * k, dir = c.clockwise ? 1 : -1;
    // A break (a node with no ink) is a point on the circle; a cycle with no nodes is one whole circle, broken at the top.
    const point = (a: number): Box => { const q: P = [centre[0] + R * Math.cos(-a), centre[1] + R * Math.sin(-a)]; return [q[0], q[1], q[0], q[1]]; };
    const said = (i: number) => { const t = textOn(m, c, i); return t ? text(t.id) : undefined; };
    const nodes = c.nodes.length ? c.nodes : [[]], boxes = nodes.map((n, i) => (n.length ? ink(n) : said(i)) ?? (c.at?.[i] !== undefined ? point(c.at[i]) : !c.nodes.length ? point(Math.PI / 2) : undefined));
    const angle = (b: Box | undefined, i: number) => (b ? Math.atan2((b[1] + b[3]) / 2 - centre[1], (b[0] + b[2]) / 2 - centre[0]) : -Math.PI / 2 + (dir * 2 * Math.PI * i) / nodes.length);
    const angles = boxes.map(angle);
    const at = (a: number, r = R): P => [centre[0] + r * Math.cos(a), centre[1] + r * Math.sin(a)];
    const gap = GAP_PT * f * pt, clear = (c.gap ?? GAP_PT) * f * pt;
    const inside = (p: P, b: Box | undefined) => !!b && p[0] > b[0] - clear && p[0] < b[2] + clear && p[1] > b[1] - clear && p[1] < b[3] + clear;
    const toDoc = (p: P): [number, number] => [p[0] / k, -p[1] / k];
    for (let s = 0; s < nodes.length; s++) {
      const t = (s + 1) % nodes.length;
      let a0 = angles[s], a1 = angles[t];
      // Travel the way the cycle turns; a two-node cycle goes half way round each time.
      let span = dir * (a1 - a0);
      while (span <= 0) span += 2 * Math.PI;
      // Leave each intermediate's ink, then stop short of the next one's.
      const step = 0.01;
      let from = 0, to = span;
      while (from < span / 2 && inside(at(a0 + dir * from), boxes[s])) from += step;
      while (to > span / 2 && inside(at(a0 + dir * to), boxes[t])) to -= step;
      if (to - from < 0.05) continue;
      out.spans.push({ cycle: c.id, step: s, centre, R, a0, dir, from, to });
      const A = at(a0 + dir * from), B = at(a0 + dir * to), mid = at(a0 + dir * (from + to) / 2);
      const style = { cycle: c.id, step: s, ...(c.colour ? { colour: c.colour } : {}), ...(c.order ? { order: c.order } : {}), ...own, ...(c.width ? { width: c.width } : {}) };
      out.arrows.push({ id: next--, kind: "curved", from: toDoc(A), to: toDoc(B), bend: bendFor(A, B, mid), ...style });
      // Side arrows: a reagent joins the arc before its middle; a product leaves after it. A step's name sits by it.
      const texts = (m.captions ?? []).filter((x) => x.cycle && x.cycle.cycle === c.id && x.cycle.step === s && onStep(x.cycle)), sides = texts.filter((x) => isSide(x.cycle));
      // A reagent and a product on the same step are one arrow, unless either is kept apart.
      const pairIn = sides.find((x) => x.cycle!.side === "in" && x.cycle!.through !== false), pairOut = sides.find((x) => x.cycle!.side === "out" && x.cycle!.through !== false);
      const paired = !!pairIn && !!pairOut;
      const w = Math.min(0.2, 0.6 / (to - from)), along = (g: number, r = R) => at(a0 + dir * (from + (to - from) * g), r);
      const clamp = (g: number) => Math.min(0.98, Math.max(0.02, g));
      // Where along the step a side arrow's end is: its own place, or for one passing through, THROUGH from its touch.
      const half = Math.min(0.45, (THROUGH * f * k) / (R * (to - from)));
      const gOf = (on: CycleText) => on.at ?? (on.side === "name" || !(paired || on.through) ? sideAt(on) : on.side === "in" ? (paired ? 0.5 : 0.4) - half : (paired ? 0.5 : 0.6) + half);
      const endOf = (on: CycleText) => along(gOf(on), R + sideReach(on) * f * k);
      const doc = (p: P): P => [p[0] / k, -p[1] / k];
      // A pass-through arrow: from one outer end, touching the arc at touch, to the other.
      const through = (start: number, end: number, ga: number, gb: number, touch: number, ids: { caption?: number; captionEnd?: number }) => {
        const A = along(ga, R + start * f * k), B = along(gb, R + end * f * k), T = along(touch);
        out.arrows.push({ id: next--, kind: "curved", from: toDoc(A), to: toDoc(B), bend: bendFor(A, B, T), ...style, ...ids });
        return T;
      };
      if (pairIn && pairOut) {
        const a = pairIn.cycle!, b = pairOut.cycle!, ga = gOf(a), gb = gOf(b);
        const T = through(sideReach(a), sideReach(b), ga, gb, clamp(a.meet ?? b.meet ?? (ga + gb) / 2), { caption: pairIn.id, captionEnd: pairOut.id });
        out.handles.set(pairIn.id, { end: doc(endOf(a)), meet: doc(T) });
        out.handles.set(pairOut.id, { end: doc(endOf(b)), meet: doc(T) });
      }
      for (const cap of texts) {
        const on = cap.cycle!;
        // A step's name goes outside its arc (clear of any side arrow's curve), or inside it where an arrow passes
        // through that step.
        const g = gOf(on), reach = on.side === "name" && on.reach === undefined ? (paired || sides.some((x) => x.cycle!.through) ? -1 : sides.length ? 0.9 : 0.3) : sideReach(on), away = along(g, R + reach * f * k);
        if (on.side === "name" || (paired && (cap === pairIn || cap === pairOut))) {
          // a step's name has no arrow; the pair's arrow is drawn above
        } else if (on.through) {
          // A tail: the arrow touches the arc and goes out again, by default as far the other way.
          const [ta, tr] = on.tail ?? [clamp(on.side === "in" ? g + 2 * (on.at === undefined ? half : w) : g - 2 * (on.at === undefined ? half : w)), reach];
          const touch = clamp(on.meet ?? (g + ta) / 2);
          const T = on.side === "in" ? through(reach, tr, g, ta, touch, { caption: cap.id }) : through(tr, reach, ta, g, touch, { captionEnd: cap.id });
          out.handles.set(cap.id, { end: doc(away), meet: doc(T), tail: doc(along(ta, R + tr * f * k)) });
        } else {
          // The arrow meets the arc 0.2 of the step on from its outer end (back from it, for a product), and no more
          // than 0.6 radians on, so it stays short on a step most of the way round (one with fewer than two nodes). A
          // reagent's arrow runs into the arc with no head, as in the literature; a product's leaves it with one.
          const fa = Math.min(0.95, Math.max(0.05, on.meet ?? (on.side === "in" ? g + w : g - w))), onArc = along(fa);
          const [p, q] = on.side === "in" ? [away, onArc] : [onArc, away];
          const bulge = along((fa + g) / 2, R + reach * (0.75 / 1.1) * f * k);
          out.arrows.push({ id: next--, kind: "curved", from: toDoc(p), to: toDoc(q), bend: bendFor(p, q, bulge) * 0.6, ...style, ...(on.side === "in" ? { caption: cap.id, end: "none" as const } : { captionEnd: cap.id }) });
          out.handles.set(cap.id, { end: doc(away), meet: doc(onArc) });
        }
        // The text sits beyond the outer end of the side arrow (a step's name, at its place): away from the circle
        // outside it, towards the middle inside it.
        const outward = Math.atan2(away[1] - centre[1], away[0] - centre[0]), textAt = at(outward, R + reach * f * k + (on.side === "name" ? 0 : Math.sign(reach || 1) * gap));
        const facing = reach < 0 ? outward + Math.PI : outward, cx = Math.cos(facing), sy = Math.sin(facing);
        const anchor = cx > 0.35 ? "start" : cx < -0.35 ? "end" : "middle";
        out.anchors.set(cap.id, { x: textAt[0], y: textAt[1] + (sy > 0.35 ? 10 : sy < -0.35 ? -2 : 3.5) * f * pt, anchor });
      }
    }
    // Structures as reagents and products: each on the step beside its middle, with an arrow from where it is to the
    // arc (a reagent's, with no head) or from the arc to it (a product's).
    for (const sd of c.sides ?? []) {
      const b = ink(sd.atoms);
      if (!b) continue;
      const mid: P = [(b[0] + b[2]) / 2, (b[1] + b[3]) / 2], a = Math.atan2(mid[1] - centre[1], mid[0] - centre[0]);
      const span = out.spans.find((x) => { if (x.cycle !== c.id) return false; let d = x.dir * (a - x.a0); while (d < 0) d += 2 * Math.PI; return d >= x.from && d <= x.to; });
      if (!span) continue;
      let d = span.dir * (a - span.a0);
      while (d < 0) d += 2 * Math.PI;
      const g = (d - span.from) / (span.to - span.from), w = Math.min(0.2, 0.6 / (span.to - span.from));
      const along = (q: number, r = R) => at(span.a0 + span.dir * (span.from + (span.to - span.from) * q), r);
      const onArc = along(Math.min(0.95, Math.max(0.05, sd.side === "in" ? g + w : g - w)));
      // The arrow's outer end: from the structure's middle towards the arc, just clear of its ink.
      const len = Math.hypot(onArc[0] - mid[0], onArc[1] - mid[1]) || 1, u: P = [(onArc[0] - mid[0]) / len, (onArc[1] - mid[1]) / len];
      let t = 0;
      while (t < len && mid[0] + u[0] * t > b[0] - gap && mid[0] + u[0] * t < b[2] + gap && mid[1] + u[1] * t > b[1] - gap && mid[1] + u[1] * t < b[3] + gap) t += 0.5;
      if (len - t < 2 * gap) continue; // the structure touches the arc: no room for an arrow
      const end: P = [mid[0] + u[0] * t, mid[1] + u[1] * t], r = Math.hypot(end[0] - centre[0], end[1] - centre[1]);
      const [p, q] = sd.side === "in" ? [end, onArc] : [onArc, end];
      const bulge = along((Math.min(0.95, Math.max(0.05, g)) + Math.min(0.95, Math.max(0.05, sd.side === "in" ? g + w : g - w))) / 2, R + (r - R) * 0.68);
      out.arrows.push({ id: next--, kind: "curved", from: toDoc(p), to: toDoc(q), bend: bendFor(p, q, bulge) * 0.6, cycle: c.id, step: span.step, ...(c.colour ? { colour: c.colour } : {}), ...(c.order ? { order: c.order } : {}), ...own, ...(c.width ? { width: c.width } : {}), ...(sd.side === "in" ? { end: "none" as const } : {}) });
    }
    for (const cap of m.captions ?? []) if (cap.cycle?.cycle === c.id && cap.cycle.side === "centre") out.anchors.set(cap.id, { x: centre[0], y: centre[1] + 3.5 * f * pt, anchor: "middle" });
  }
  return out;
}

// The bend (see render.ts) of a curve from A to B through M: M's offset to the left of A→B, as a fraction of |AB|.
// Points are in px with y down, where n is the left of A→B as drawn: the same side as in drawing coordinates.
function bendFor(A: P, B: P, M: P): number {
  const len = Math.hypot(B[0] - A[0], B[1] - A[1]) || 1, n: P = [(B[1] - A[1]) / len, -(B[0] - A[0]) / len];
  // arrowPath draws the arc through M, so the cycle's arrows follow its circle.
  return ((M[0] - (A[0] + B[0]) / 2) * n[0] + (M[1] - (A[1] + B[1]) / 2) * n[1]) / len;
}

// Where each intermediate should go: evenly round a circle, starting at the top and turning the cycle's way, on a
// radius big enough that no two intermediates come closer than about two bonds. Returns each node's shift (drawing
// units) and the radius (bond lengths).
export function spreadCycle(c: Cycle, k: number, ink: (atoms: number[], i: number) => Box | undefined): { shifts: P[]; radius: number } {
  const n = c.nodes.length, boxes = c.nodes.map((x, i) => ink(x, i));
  const size = Math.max(...boxes.map((b) => (b ? Math.hypot(b[2] - b[0], b[3] - b[1]) / 2 : k)));
  const radius = Math.max(2.5 * k, (2 * size + 2 * k) / (2 * Math.sin(Math.PI / Math.max(n, 2)))) / k;
  const centre: P = [c.centre[0] * k, -c.centre[1] * k];
  const shifts = boxes.map((b, i): P => {
    const a = -Math.PI / 2 + ((c.clockwise ? 1 : -1) * 2 * Math.PI * i) / n;
    const target: P = [centre[0] + radius * k * Math.cos(a), centre[1] + radius * k * Math.sin(a)];
    const now: P = b ? [(b[0] + b[2]) / 2, (b[1] + b[3]) / 2] : target;
    return [(target[0] - now[0]) / k, -(target[1] - now[1]) / k];
  });
  return { shifts, radius };
}

// Orders intermediates by their angle around their middle, clockwise from the top (as they read on screen).
export function orderAround(centre: P, items: { atoms: number[]; middle: P }[]): number[][] {
  const angle = (p: P) => { const a = Math.atan2(-(p[1] - centre[1]), p[0] - centre[0]); return (a + Math.PI / 2 + 4 * Math.PI) % (2 * Math.PI); };
  return [...items].sort((a, b) => angle(a.middle) - angle(b.middle)).map((x) => x.atoms);
}

export const isCycleText = (c: Caption) => !!c.cycle;

// The Cycle tool. A structure's middle (drawing units): the middle of its atoms' box.
const middleOf = (m: Mol, atoms: number[]): P | undefined => {
  const ps = atoms.flatMap((i) => (m.atoms[i] ? [[m.atoms[i].location[0], m.atoms[i].location[1]]] : []));
  if (!ps.length) return undefined;
  const xs = ps.map((q) => q[0]), ys = ps.map((q) => q[1]);
  return [(Math.min(...xs) + Math.max(...xs)) / 2, (Math.min(...ys) + Math.max(...ys)) / 2];
};
// The text written as node i of a cycle.
export const textOn = (m: Mol, c: Cycle, i: number) => m.captions?.find((t) => t.cycle?.cycle === c.id && t.cycle.side === "node" && t.cycle.step === i);
export const angleAbout = (c: Cycle, p: P) => Math.atan2(p[1] - c.centre[1], p[0] - c.centre[0]);
// Each node's angle: where its structure is, or its saved angle.
const anglesOf = (m: Mol, c: Cycle) => c.nodes.map((n, i) => { const t = n.length ? undefined : textOn(m, c, i), q = t ? [t.x, t.y] as P : middleOf(m, n); return q ? angleAbout(c, q) : (c.at?.[i] ?? Math.PI / 2); });
// A cycle with every node's angle saved as it is now (before its atoms go).
export const keepAngles = (m: Mol, c: Cycle): Cycle => ({ ...c, at: anglesOf(m, c) });
// How far round from node 0 an angle is, going the way the cycle turns (clockwise on screen is down in angle).
const round = (c: Cycle, from: number, a: number) => { const d = (c.clockwise ? from - a : a - from) % (2 * Math.PI); return d < 0 ? d + 2 * Math.PI : d; };
// The step an angle is on: node k to k + 1. A cycle with no nodes has one step.
export function stepAt(m: Mol, c: Cycle, a: number): number {
  const angles = anglesOf(m, c);
  if (angles.length < 2) return 0;
  const t = round(c, angles[0], a), next = angles.findIndex((x, k) => k > 0 && round(c, angles[0], x) > t);
  return next < 0 ? angles.length - 1 : next - 1;
}
// The break nearest an angle, within tol radians.
export function breakAt(m: Mol, c: Cycle, a: number, tol: number): number | undefined {
  const angles = anglesOf(m, c);
  let best: number | undefined, d = tol;
  c.nodes.forEach((n, k) => { if (n.length || textOn(m, c, k)) return; const e = Math.abs(((angles[k] - a + 3 * Math.PI) % (2 * Math.PI)) - Math.PI); if (e <= d) { d = e; best = k; } });
  return best;
}
const withCycle = (m: Mol, c: Cycle, shift: (step: number) => number): Mol => ({
  ...m,
  cycles: m.cycles?.map((x) => (x.id === c.id ? c : x)),
  captions: m.captions?.map((t) => (t.cycle?.cycle === c.id && t.cycle.side !== "centre" && shift(t.cycle.step) !== t.cycle.step ? { ...t, cycle: { ...t.cycle, step: shift(t.cycle.step) } } : t)),
});
// A node (a structure, or a break when atoms is empty) at an angle, in its place going round. The step it lands on
// splits in two: its reagents and products stay on the first half, and the steps after it move on by one.
export function addStop(m: Mol, id: number, a: number, atoms: number[]): Mol {
  const c = m.cycles?.find((x) => x.id === id);
  if (!c) return m;
  const angles = anglesOf(m, c), j = c.nodes.length ? stepAt(m, c, a) + 1 : 0;
  const nodes = [...c.nodes.slice(0, j), atoms, ...c.nodes.slice(j)], at = [...angles.slice(0, j), a, ...angles.slice(j)];
  return withCycle(m, { ...c, nodes, at }, (s) => (c.nodes.length > 1 && s >= j ? s + 1 : s));
}
// Takes node j out; its two steps become one.
export function removeStop(m: Mol, id: number, j: number): Mol {
  const c = m.cycles?.find((x) => x.id === id);
  if (!c) return m;
  const angles = anglesOf(m, c), freed = { ...m, captions: m.captions?.map((t) => (t.cycle?.cycle === id && t.cycle.side === "node" && t.cycle.step === j ? (({ cycle: _, ...rest }) => rest)(t) : t)) };
  return withCycle(freed, { ...c, nodes: c.nodes.filter((_, k) => k !== j), at: angles.filter((_, k) => k !== j) }, (s) => (s >= j && s > 0 ? s - 1 : s));
}
// How close a structure's middle must be to a circle to be on it (bond lengths): a bond length, or more for a big one.
const onCircle = (m: Mol, c: Cycle, atoms: number[]) => {
  const q = middleOf(m, atoms);
  if (!q) return false;
  const xs = atoms.map((i) => m.atoms[i].location[0]), ys = atoms.map((i) => m.atoms[i].location[1]), size = Math.hypot(Math.max(...xs) - Math.min(...xs), Math.max(...ys) - Math.min(...ys)) / 2;
  const d = Math.hypot(q[0] - c.centre[0], q[1] - c.centre[1]);
  return d > 0.35 * c.radius && Math.abs(d - c.radius) <= Math.max(1, 0.75 * size);
};
// Structures and text on a cycle's circle become its intermediates: one that lands on a break takes its place. moved:
// the atoms an edit moved or added (every atom when unset), so only those structures join; movedText: the captions it
// moved or added (every one when moved is unset). Only free text joins (not a label, condition or a cycle's own).
// A structure that is in no cycle joins the first whose circle it is on.
export function joinCycles(m: Mol, frags: number[][], moved?: (i: number) => boolean, movedText?: (c: Caption) => boolean): Mol {
  if (!m.cycles?.length) return m;
  const taken = new Set(m.cycles.flatMap((c) => c.nodes.flat()));
  for (const f of frags) {
    if (f.some((i) => taken.has(i)) || (moved && !f.some(moved))) continue;
    const c = m.cycles!.find((x) => onCircle(m, x, f));
    if (!c) continue;
    const a = angleAbout(c, middleOf(m, f)!), xs = f.map((i) => m.atoms[i].location[0]), ys = f.map((i) => m.atoms[i].location[1]);
    const spot = breakAt(m, c, a, Math.max(0.6, Math.hypot(Math.max(...xs) - Math.min(...xs), Math.max(...ys) - Math.min(...ys)) / 2 + 0.5) / c.radius);
    m = spot !== undefined ? withCycle(m, { ...c, nodes: c.nodes.map((n, k) => (k === spot ? f : n)), at: anglesOf(m, c).map((x, k) => (k === spot ? a : x)) }, (s) => s) : addStop(m, c.id, a, f);
    f.forEach((i) => taken.add(i));
  }
  for (const t of m.captions ?? []) {
    if (t.under || t.on || t.cycle || (moved && !movedText?.(t))) continue;
    const c = m.cycles!.find((x) => { const d = Math.hypot(t.x - x.centre[0], t.y - x.centre[1]); return d > 0.35 * x.radius && Math.abs(d - x.radius) <= 0.75; });
    if (!c) continue;
    const a = angleAbout(c, [t.x, t.y]), spot = breakAt(m, c, a, 0.8 / c.radius), j = spot ?? (c.nodes.length ? stepAt(m, c, a) + 1 : 0);
    if (spot === undefined) m = addStop(m, c.id, a, []);
    else m = { ...m, cycles: m.cycles!.map((x) => (x.id === c.id ? { ...x, at: anglesOf(m, x).map((y, k) => (k === spot ? a : y)) } : x)) };
    m = { ...m, captions: m.captions!.map((x) => (x.id === t.id ? { ...x, cycle: { cycle: c.id, step: j, side: "node" } } : x)) };
  }
  return m;
}

// Where a side arrow's text dragged to p (px, y down) puts its outer end: how far along its step and how far from the
// circle (CycleText at and reach). Undefined when the step is not drawn.
export function sideFor(layout: CycleLayout, t: CycleText, p: P, k: number, f = 1): { at: number; reach: number } | undefined {
  const s = layout.spans.find((x) => x.cycle === t.cycle && x.step === t.step);
  if (!s) return undefined;
  let travel = (s.dir * (Math.atan2(p[1] - s.centre[1], p[0] - s.centre[0]) - s.a0)) % (2 * Math.PI);
  if (travel < 0) travel += 2 * Math.PI;
  // Past the end of the step is its end; before its start (the rest of the way round) is its start.
  const past = travel > s.to + (2 * Math.PI - s.to + s.from) / 2 ? 0 : Math.min(1, Math.max(0, (travel - s.from) / (s.to - s.from)));
  const reach = (Math.hypot(p[0] - s.centre[0], p[1] - s.centre[1]) - s.R) / (f * k);
  // The end stays a little away from the circle, and inside it no nearer the middle than a fifth of the radius.
  return { at: Math.round(past * 100) / 100, reach: Math.round(Math.max(-0.8 * s.R / (f * k), Math.abs(reach) < 0.4 ? Math.sign(reach || 1) * 0.4 : reach) * 100) / 100 };
}

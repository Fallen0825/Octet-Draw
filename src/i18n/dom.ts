// 비공식 한글판: 화면에 그려진 영어 글자를 한글로 바꿉니다 (ko.json의 menu, ui, patterns).
// 원본 코드는 그대로 두고, 화면(DOM)에 글자가 나타나거나 바뀔 때마다 바꿔 씁니다. 그래서 원본의 새 버전을
// 합칠 때 충돌이 적습니다. React가 글자를 다시 쓰면 MutationObserver가 다시 바꿉니다.
// 바꾸지 않는 곳: 그림(svg.canvas), 글을 입력하는 곳(input, textarea, contenteditable), translate="no".
import ko from "./ko.json" with { type: "json" };

type Pattern = { re: RegExp; ko: string };

const words = new Map<string, string>();
for (const section of [ko.menu, ko.ui] as Record<string, string>[]) {
  for (const [en, k] of Object.entries(section)) words.set(en.toLowerCase(), k);
}
const escape = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
const patterns: Pattern[] = Object.entries(ko.patterns as Record<string, string>).map(([en, k]) => ({
  re: new RegExp("^" + en.split(/\{\d\}/).map(escape).join("(.+?)") + "$", "i"),
  ko: k,
}));

const HANGUL = /[가-힣]/;
const LETTERS = /[A-Za-z]/;

// A word inside a pattern ("Paper" in "New Paper"): its Korean alone.
function inner(s: string): string {
  const k = words.get(s.trim().toLowerCase());
  return k === undefined ? s : k.startsWith("!") ? k.slice(1) : k;
}

// The Korean for an English text, or undefined when there is none.
// compact: a narrow place (a menu's item, a chip), where the English would be cut off: Korean only.
export function translate(text: string, compact = false): string | undefined {
  const core = text.trim();
  if (!core || !LETTERS.test(core) || HANGUL.test(core) || core.length > 400) return undefined;
  const lead = text.slice(0, text.indexOf(core)), trail = text.slice(text.indexOf(core) + core.length);
  let base = core, ellipsis = "";
  if (base.endsWith("…")) [base, ellipsis] = [base.slice(0, -1), "…"];
  else if (base.endsWith("...")) [base, ellipsis] = [base.slice(0, -3), "…"];
  const k = words.get(base.toLowerCase());
  if (k !== undefined) {
    let out: string;
    if (k.startsWith("!")) out = k.slice(1);
    else if (compact || /[.?!:]$/.test(base)) out = k; // a sentence or a narrow place: Korean only
    else if (base.includes("(")) out = `${k} — ${base}`; // "의자형 (뒤집음) — Chair (flipped)", not nested brackets
    else out = `${k} (${base})`;
    return lead + out + ellipsis + trail;
  }
  for (const p of patterns) {
    const m = core.match(p.re);
    if (m) return lead + p.ko.replace(/\{(\d)\}/g, (_, i) => inner(m[Number(i) + 1] ?? "")) + trail;
  }
  return undefined;
}

// Not changed at all: the drawing and what people type into. A text field's own words (its placeholder and title)
// are changed, but never what is typed in it. Nor the names of drawings and folders on Home, which are file names
// ("Paper.oct" stays "Paper"); Home's New tiles (Quick start, Paper…) are Octet's own and are changed.
const SKIP = "svg.canvas, script, style, [contenteditable=''], [contenteditable='true'], [translate='no']";
const TYPED = "input, textarea, .home-tile:not(.new, .new-project) .home-name";
const ATTRS = ["title", "placeholder"];
// Narrow places: the names in menus (right-click menus and the tool menus), the chips in the Actions panel, and Home's
// New tiles and their heading.
const COMPACT = ".variant-name, .picks, .chip, .home-tile.new .home-name, .home-tile.new-project .home-name, .home-heading[data-heading='new']";
// Names kept to one line, with the whole name in a tooltip when it is cut (ko.css).
const ONE_LINE = ".action-name";

const skipped = (el: Element | null) => !el || !!el.closest(SKIP);

function textNode(n: Text) {
  const parent = n.parentElement;
  if (skipped(parent) || parent!.closest(TYPED)) return;
  const k = translate(n.nodeValue ?? "", !!parent!.closest(COMPACT));
  if (k !== undefined && k !== n.nodeValue) n.nodeValue = k;
  fullTip(parent!);
}

// An Actions panel row shows its name on one line (ko.css), cut short with … when it is long. The app's tooltip then
// shows the whole name, only while it is cut (data-tip-cut, Tooltip.tsx).
function fullTip(parent: Element) {
  if (!parent.matches(ONE_LINE)) return;
  const row = parent.closest("button");
  const text = parent.textContent?.trim();
  if (!row || !text || row.getAttribute("data-tip") === text) return;
  row.setAttribute("data-tip", text);
  row.setAttribute("data-tip-cut", "");
}

function attribute(el: Element, name: string) {
  const v = el.getAttribute(name);
  if (!v) return;
  const k = translate(v);
  if (k !== undefined && k !== v) el.setAttribute(name, k);
}

function element(el: Element) {
  if (skipped(el)) return;
  for (const a of ATTRS) attribute(el, a);
  const walker = document.createTreeWalker(el, NodeFilter.SHOW_TEXT | NodeFilter.SHOW_ELEMENT, {
    acceptNode: (n) => (n instanceof Element && n.matches(SKIP) ? NodeFilter.FILTER_REJECT : NodeFilter.FILTER_ACCEPT),
  });
  for (let n = walker.nextNode(); n; n = walker.nextNode()) {
    if (n.nodeType === Node.TEXT_NODE) textNode(n as Text);
    else for (const a of ATTRS) attribute(n as Element, a);
  }
}

export function startKorean(root: HTMLElement = document.body) {
  document.documentElement.lang = "ko";
  element(root);
  new MutationObserver((records) => {
    for (const r of records) {
      if (r.type === "characterData") textNode(r.target as Text);
      else if (r.type === "attributes") { if (!skipped(r.target as Element)) attribute(r.target as Element, r.attributeName!); }
      else for (const n of r.addedNodes) {
        if (n.nodeType === Node.TEXT_NODE) textNode(n as Text);
        else if (n.nodeType === Node.ELEMENT_NODE) element(n as Element);
      }
    }
  }).observe(root, { subtree: true, childList: true, characterData: true, attributes: true, attributeFilter: ATTRS });
}

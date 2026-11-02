# CLAUDE.md

Guidance for Claude Code sessions in this repository, both local on Sam's
Mac and in cloud sessions.

## Read first

1. [OCTET_BRIEF.md](OCTET_BRIEF.md): product, scope, build order and
   definition of done. Read it fully before any work.
2. [brand/CLAUDE.md](brand/CLAUDE.md) and [brand/BRAND.md](brand/BRAND.md)
   before any UI, icon or user-facing text work. Every value lives in
   `brand/tokens.json` and `brand/tokens.css`. There is no generator yet, so
   change both and keep them in step.
3. [docs/licence.md](docs/licence.md) before you add a dependency or
   anything that comes from studying ChemDraw.
4. [docs/decisions/](docs/decisions/): decisions that change the brief.

## Project

Octet is a free, open-source (Apache-2.0), native, offline desktop app for
drawing chemical structures. It replaces ChemDraw. Platforms: Mac, Windows and
Linux. Status: brief build step 1 (foundation).

Stack: Tauri 2 (Rust) shell, React + TypeScript + Vite for the UI, and
Indigo (`indigo-ketcher`, WASM) for chemistry and CDX/CDXML (see
[docs/decisions/0001-chemistry-engine.md](docs/decisions/0001-chemistry-engine.md)).
Octet draws structures itself as SVG. The app icon source is
`src-tauri/icons/app-icon.svg`.

## Hard rules (from the brief)

- Free for every individual. No trial timers, watermarks, export limits,
  licence keys, sign-up gates, upsell modals or Pro locks.
- Offline-first. Every feature works with no network. No account to start:
  Octet opens to a Home page of recent drawings, where Return starts a new
  one (decision 0003). Accounts are optional and only add sync and sharing.
  Files always save locally, as `.oct` (CDXML inside, decision 0004).
- Signed and notarised installers.
- CDXML import and export from version 1, with a round-trip test suite.
- New documents use ACS Document 1996 by the book (Arial 10 pt labels).
  Users can change the style. Export matches the page exactly.
- When a question in section 11 of the brief blocks you, ask Sam. Don't
  guess.
- Cross-check ChemDraw for every feature, before you build it and again
  before you show it. List every option ChemDraw gives that tool or object:
  its palette, the Object, Curves, Color and Text menus, the context menu,
  its dialogs, and the attributes its CDXML object takes. Run ChemDraw
  26.1 (the baseline, decision 0006) on Sam's Mac and try each one (ask Sam for computer-use access),
  and read its public documentation. Octet gets every option, or Sam gets a list of the ones missing.
  Never guess and never skip the check.

Colours, fonts, British English, STE and accessibility rules are in
`brand/CLAUDE.md`.

## Product priorities

Two pillars come before every feature:

- **Consistency.** A behaviour that works in one place works in every place
  it could apply. Every tool holds to Select, every object takes a colour,
  ⌘A selects every kind of object, and every menu is the same component.
  When you add or change a behaviour, apply it to every tool, object, menu
  and panel it fits, and test each one.
- **ChemDraw cross-check, above everything.** See the hard rule above. If
  ChemDraw lets you do something to an object, Octet must too.

The top priority is that Octet is a pleasure to use: super smooth and
snappy with no lag, well designed and really well animated (within the
motion rules in `brand/`), better looking than ChemDraw, with better
features and good templates. It caters for everyone. It isn't daunting at
first sight, but it has a huge depth of features for people who go looking.
Final Cut Pro is the model for this.

Sam's additions to the brief (14 October 2026), in no strict order:

- Paste into Word, PowerPoint and Keynote. Version 1 mirrors ChemDraw's
  clipboard: a vector PDF at true size with fonts embedded and the
  drawing's CDXML in the PDF's metadata stream, then CDX, MOL and SMILES.
  ChemDraw already puts the CDXML in its PDF, and Word hands the PDF back
  unchanged, so a picture copied back out of Word pastes as an editable
  structure in Octet and in ChemDraw. Later: atom recolouring inside
  Office.
- Keyboard drawing as fast as ChemDraw's hotkeys.
- Crash safety: autosave, recovery and atomic saves.

Ideas to explore:

- A radial menu at the cursor, like Vorssaint's: hold a key or mouse
  button, point, release. Use it for quick molecules and templates, and
  quick actions such as clean-up. Keep item positions fixed so experts can
  flick without looking, as in Maya's marking menus and Blender's pie
  menus.

## Security

Security is a top priority.

- No account required, no licence checks, no telemetry. Crash reports and
  sync are opt-in. The auto-update default is an open question (brief
  section 11).
- Treat every opened or pasted file as hostile. Parse it with bounds
  checks and limits on size and nesting. No `unsafe` Rust for parsing.
- Keep the CSP in `src-tauri/tauri.conf.json` strict. No remote scripts,
  styles or fonts.
- Grant Tauri capabilities per feature, with the narrowest scope that
  works (for example, file access only to files the user picked).
- Never pass file or clipboard content to `eval`, `innerHTML` or
  `dangerouslySetInnerHTML`.
- `.npmrc` blocks install scripts. Commit lockfiles. Run `npm audit` and
  `cargo audit` before each release. Check the maintainer and licence of
  every new dependency.
- Releases: signed and notarised installers, and signed updates.

## How to build

- Code: use `/ponytail ultra`. Write the smallest change that works.
  Ponytail never removes accessibility, data safety or anything Sam asked
  for.
- UI: load `/impeccable`, `/emil-design-eng` and `/design-taste-frontend`
  before UI work. Measure interactions against the best apps for each job:
  Apple's Human Interface Guidelines for Mac conventions, Final Cut Pro for
  a simple surface over deep features, Figma and Sketch for canvas
  selection, snapping, zoom and nudging, and Linear and Things for calm,
  keyboard-first chrome.
- When they disagree: `brand/` decides look and copy, then accessibility,
  then the design skills, then ponytail on code size.
- These skills are installed on Sam's Mac only. Cloud sessions follow
  `brand/`.
- Record significant technical decisions in `docs/decisions/`.
- Check every feature against ChemDraw side by side (same input, compare
  the output, the speed and the number of steps) so Octet is never worse.

## Licences

- Add only dependencies with permissive licences: MIT, BSD, Apache-2.0,
  ISC. Fonts may use OFL-1.1. Ask Sam before you add anything else.
- Never add GPL or AGPL code, for example Open Babel.
- Record each dependency in [THIRD_PARTY.md](THIRD_PARTY.md).

## ChemDraw: what we can use

Octet takes ChemDraw's functionality as the minimum and builds on it, with
its own look and feel. Study how ChemDraw does something, then design a
better, modern way to do it. This repository is public.

- Free to use: ChemDraw's behaviour, workflow logic, keyboard shortcuts and
  file formats. Functionality and data file formats are not protected by
  copyright (SAS Institute v World Programming, CJEU, 2012). Learn them by
  using and testing ChemDraw, and from its public documentation.
- Never put ChemDraw's code, icons, templates, documentation text or sample
  files in this repository. Write our own code.
- Learn what a feature does, never how ChemDraw codes it, and design
  Octet's own way.
- Use only files Sam created as test files. Some of Sam's corpus files
  contain ChemDraw template clip art or third-party pictures: never commit
  those. Keep corpus tests that need them local.

## ChemDraw on Sam's Mac

- ChemDraw 26.1.0, the baseline (decision 0006), is at
  `/Applications/ChemDraw.app`.
- ChemDraw 22.2.0 is at `/Applications/ChemDraw 22.2.0.app`.
- Cloud sessions have no ChemDraw.

## Workflow

1. Pull before you start. Local and cloud sessions share this repository
   through GitHub.
2. Make the change.
3. Show Sam a preview: run the app so its window opens on Sam's Mac, or
   show the diff when there is nothing to run.
4. When Sam approves, commit and push straight to `main`. No branches or
   pull requests are needed. This is standing permission from Sam.

Never force-push or rewrite history on `main`.

Preview command: `npm run tauri dev` (needs Node and Rust). In the Claude
app, start the `octet` preview from `.claude/launch.json`: it opens the app
window and shows the UI in the preview pane. Cloud sessions without Rust use
the `octet-ui-only` preview (`npm run dev`, http://localhost:1420).

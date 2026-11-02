# Octet Draw

Free, open-source chemical drawing for Mac, Windows and Linux. It opens and saves ChemDraw files, and it works
with no internet connection.

Octet Draw is part of Octet, the Open source Chemistry Toolkit for Experiments & Typesetting. Made by
[Sam Ellis](https://www.linkedin.com/in/sam-ellis-132781206/), with Claude.

## Download

Get the latest version from [octet.software](https://octet.software) or the
[Releases page](https://github.com/SamChemMan/Octet-Draw/releases/latest).

| Computer | File |
| -------- | ---- |
| Mac | `Octet-Draw-mac.dmg` (Apple silicon and Intel) |
| Windows | `Octet-Draw-windows.exe`. For a PC with no internet: `Octet.Draw_<version>_x64-offline-setup.exe` |
| Linux | `.deb` (Ubuntu, Debian), `.rpm` (Fedora) or `Octet-Draw-linux.AppImage` |

To uninstall: on a Mac, drag Octet Draw from Applications to the Bin. On Windows, Settings > Apps > Installed
apps > Octet Draw > Uninstall. On Linux, remove the package with your package manager, or delete the AppImage.

## System requirements

- macOS 12 Monterey or later, on Apple silicon or Intel.
- Windows 10 or 11, 64-bit.
- Linux, 64-bit, with WebKitGTK 4.1 (for example Ubuntu 22.04 or Debian 12, or later).

Octet needs no account and no internet connection.

## What it does

- Draws structures, reactions, arrows, shapes and text, with ChemDraw's keyboard shortcuts.
- Starts each drawing in the ACS Document 1996 style. Settings has other journal styles.
- Opens and saves ChemDraw files (CDXML and CDX). Imports and exports MOL, SDF, SMILES and InChI.
- Exports PDF, SVG and PNG. On the Mac, Copy puts the drawing into Word, PowerPoint and Keynote, and it pastes
  back into Octet or ChemDraw as a structure.
- Turns names into structures and structures into names, offline.
- Predicts ¹H and ¹³C NMR shifts, and shows formula, mass and other properties.

## Community

Ask a question, suggest an idea or show what you drew in
[Discussions](https://github.com/SamChemMan/Octet-Draw/discussions).

## Report a problem

Use Help > Report a Problem in Octet, or open an [issue](https://github.com/SamChemMan/Octet-Draw/issues/new/choose).
For a security problem, follow [SECURITY.md](SECURITY.md) instead.

## Run from source

You need [Node.js](https://nodejs.org) 24 and [Rust](https://rustup.rs).

```bash
npm install
npm run tauri dev
```

`npm test` runs the type check and every check file. [docs/release.md](docs/release.md) tells how releases are
built and signed.

## Sam

Hello everyone. I'm a student in the [Nitschke](https://www.nitschkegroup-cambridge.com/) group at the University of Cambridge.
Before that, I was an undergraduate at the University of Manchester.

Chemistry is fantastic, but communicating it is extremely difficult. This is my first attempt to make it
ever-so-slightly easier.

Always up for a chat about my research, this project, or anything else.

Thanks to my wonderful pals for beta testing: Elliot Thaw, Dylan Davies-Jones, James Baron, Elena Lahr, Seb Kinzie
and Ying Hou.

Email sam[at]octet.software, or for chemistry, spe29[at]cam.ac.uk. You can also find me on
[LinkedIn](https://www.linkedin.com/in/sam-ellis-132781206/).

## Privacy

Octet works with no network: no account, no licence check, no analytics, no crash reporting.
It goes online only in these cases:

- **Update check.** When it opens, Octet asks GitHub whether a newer version exists. It sends no drawings,
  files or personal details. Turn it off in Settings > General > Updates. See
  [decision 0010](docs/decisions/0010-updates.md).
- **Search Reaxys** opens Reaxys (Elsevier) in your browser with the selected structure in the address, only
  when you press it.
- **Search PubChem** opens PubChem (the US National Library of Medicine) in your browser with the selected
  structure in the address, only when you press it.

The structure stays in your browser's history. To hide both searches, turn off
Settings > Import and export > Online search. See
[decision 0008](docs/decisions/0008-online-search.md).

## Code signing policy

GitHub Actions builds every installer from this repository. Nothing is built or signed on a personal computer.

- **Mac:** signed with an Apple Developer ID and notarised by Apple.
- **Windows:** signing is being set up. Until then, Windows may warn you the first time you open the
  installer: click More info, then Run anyway.

- Committers and reviewers: [Sam Ellis (SamChemMan)](https://github.com/SamChemMan)
- Approvers: [Sam Ellis (SamChemMan)](https://github.com/SamChemMan)

The policy is also at [octet.software/code-signing](https://octet.software/code-signing).

## Licence

Octet is free and open-source software under the
[Apache License 2.0](LICENSE).

Copyright © 2026 Sam Ellis. Made in Manchester and Cambridge.

See the [licence plan](docs/licence.md) for the rules on third-party
libraries and on studying other software.

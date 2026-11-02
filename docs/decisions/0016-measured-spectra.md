# 0016: Measured spectra, and links from their peaks to atoms and bonds

Status: built 27 October 2026 for Sam to review (roadmap 1.1.0 step 2, audit item CH-29).

## ChemDraw 26.1

From the 26.1 help (Importing, ChemNMR) and the strings of `ChemDrawBase`:

- File > Open reads JCAMP (JDX, DX) and Galactic SPC. A spectrum becomes an object on the page: NMR, IR, MS,
  UV-Vis or generic. The JCAMP labels it names are XYDATA (X++(Y..Y)), FIRSTX, LASTX, NPOINTS, XFACTOR, YFACTOR,
  XUNITS, YUNITS and DATA TYPE. A file it cannot read gives "unable to interpret this JCAMP file".
- Assignment: select atoms and bonds, Shift-click peaks, then Structure > Make Spectrum-Structure Assignment.
  Clear Spectrum-Structure Assignment removes it. With Lasso or Marquee, pointing at a peak lights its atoms.
- The spectrum object's menu: Center on Page, Bring to Front, Send to Back, Flip, Scale, and Mass Spec Analysis.
- The code picks peaks (for NMR, IR, UV-Vis and MS) so they can be Shift-clicked. Nothing in the help, the strings or
  the code's names shows multiplets, J values, integrals or a text line.

Open questions for a run of 26.1 are item 19 in `~/octet-research/notes/chemdraw-checks-pending.md`.

## What Octet does

- `src/jcamp.ts` reads JCAMP-DX 4.24 to 6.0: XYDATA in every form (AFFN, PAC, SQZ, DIF, DUP, with the Y check),
  XYPOINTS, PEAK TABLE (XY, XYW, XYM), NTUPLES pages of a processed NMR spectrum (the real part), and compound files.
  NMR in Hz becomes ppm by .OBSERVE FREQUENCY, and .SHIFT REFERENCE or $OFFSET moves the axis. It reads Bruker's
  processed data (procs, 1r and acqus) too. A raw FID and a 2D spectrum are refused with a plain message.
- Limits, checked before memory is used: 64 MB of text, 2,097,152 points in a spectrum, 8,388,608 in a file, 64
  blocks, blocks nested 3 deep, 100,000 peaks in a table, 16 spectra. A repeat count (DUP) that would pass the
  limit stops the read at once.
- `src/measured.ts` picks peaks above a threshold (2% of the tallest by default, and four times the noise), each
  standing out from its surroundings. For ¹H it groups peaks no more than 18 Hz apart into multiplets, the
  solvent's, water's and TMS's lines apart, and finds first-order patterns and their J values by Hoye's method
  (J. Org. Chem. 1994, 59, 4096). It integrates each multiplet and scales the integrals so the counts are nearest
  whole numbers, or to the count the user gives one.
- The text line follows ACS style: "¹H NMR (400 MHz, CDCl₃) δ 7.26 (d, J = 8.0 Hz, 2H), 7.35–7.20 (m, 5H)". A
  multiplet m is given by the range of its peaks, as MestReNova's multiplet report does.

## Where it is kept

- The spectrum is part of the drawing (`Mol.spectra`, one for each nucleus or kind). The drawing's JSON keeps the
  trace as 16-bit levels in base64 with the range they stand for, at most 262,144 points (a longer trace keeps the
  largest point of each run, so no peak is lost). A drawing opens with the same trace, so peak picking gives the
  same peaks after saving.
- Multiplets have ids. Each atom and bond keeps the ids of the multiplets it is linked to (`Atom.peaks`,
  `Bond.peaks`), as it keeps its highlight. So links move, copy, undo and delete with their atoms and bonds. A link to
  an id the drawing does not have is dropped when a file or a paste is read.
- A bond links as an atom does, as ChemDraw's assignment takes atoms and bonds (an IR band to its C=O): click the
  multiplet, then the bond, or drag the multiplet to the bond. A multiplet clicked with atoms selected links them and
  every bond between two of them. Pointing at the multiplet lights its bonds as a band along each; pointing at a bond
  lights its multiplets; Colour peaks colours the bonds' highlights too.
- The .oct, .cdxml and .cdx files keep the spectrum inside Octet's drawing, and also as ChemDraw's spectrum object
  (below), so ChemDraw shows it and Octet reads it back after ChemDraw saves the file.

## ChemDraw's spectrum object

From the public CDX spec (the Spectrum object, 0x8010, and its properties) and files saved by 26.1:

- CDXML `<spectrum>` has a BoundingBox, XLow and XSpacing (an even grid), XType (Wavenumbers, Microns, Hertz,
  MassUnits, PartsPerMillion, Other), YType (Absorbance, Transmittance, PercentTransmittance, Other, ArbitraryUnits),
  Class (Infrared, UVVis, MassSpectrum, NMR, Raman and others), YLow and YScale. Its text is the Y values. ChemDraw
  writes each as a whole number of YScale above YLow, 16 a line (YScale is the range over 10,000 when the file sets
  none), and reads each as a number, times YScale when YScale is not 0 or 1, plus YLow. In CDX the values are 64-bit
  floats in full (0x0A86).
- Structure > Make Spectrum-Structure Assignment puts an object tag on the spectrum and on every atom and bond
  selected: Name `/CS/CD/assign`, TagType String, its value the selected peaks' ranges as `lo-hi,` pairs. Pointing at
  a peak lights each object with a range that overlaps the peak.

What Octet does:

- Each measured spectrum is written as a spectrum object, where ChemDraw had it or else under the drawing. The grid
  runs from low x to high. A peak table's sticks go on a 0.1 grid, as ChemDraw's own mass spectra do. Uneven points
  are put on an even grid. The values are 16-bit levels (YScale is the range over 65,535), so the trace comes back
  as Octet saved it.
- Each multiplet linked to atoms or bonds is an assignment tag with the multiplet's range, on the atoms, the bonds and
  the spectrum.
- Octet's own details (the title, nucleus, frequency, solvent, threshold, integral and multiplets) go in an object tag
  named `Octet spectrum` on the spectrum, as JSON in ASCII. ChemDraw's object has no place for them.
- A ChemDraw spectrum reads as one spectrum of each kind. Without Octet's tag, an NMR spectrum's nucleus comes from
  its width (more than 40 ppm is ¹³C) and it has no frequency, so ¹H peaks are not grouped into multiplets. A mass
  spectrum that is at its lowest value at most of its points reads as sticks. Each assignment range links to the
  multiplet its middle is in, else to those it overlaps, else to a new multiplet of that range, as if drawn by hand.
- A spectrum with no grid, a value that is not a number, or more than 2,097,152 values is not read, and Octet counts
  it in what it could not open. So is a second spectrum of a kind, and an assignment with no spectrum.

## Why Octet's own reader

jcampconverter (MIT, cheminfo) reads the same forms. Octet's reader is under 350 lines, checks every limit this
project needs before it allocates, refuses a damaged file with a message the user can read, and adds no
dependency.

## Bruker

Bruker data opens by a drop (the pdata folder, its 1r or procs file, or the experiment folder) and by a pasted path.
File > Open cannot pick it: its file dialog picks files by extension, and 1r and procs have none.

## Galactic SPC

ChemDraw 26.1 opens Galactic SPC files (its code has a Galactic reader that names the 4B style). Octet reads SPC from
Galactic's published format, versions 4B (little endian) and 4C (big endian): the 512-byte header, X values when the
file gives them (its own for each subfile in an XYXY file), and Y values as 32-bit floats or as 32-bit or 16-bit whole
numbers times a power of two. It reads the first subfile of a file with several. The experiment type, else the X
units, gives the kind (FT-IR, NIR and Raman are IR; UV-Vis; MS, with an XYXY file's points as sticks; NMR in ppm); the
comment is the title.

The limits are JCAMP's: 64 MB and 2,097,152 points, each count checked against the file's length before it is read.
The old format (4D), an interferogram, a FID, an NMR spectrum not in ppm, and a value that is not a number are refused
with a plain message. The app hands an SPC file to the page as its bytes in base64 (src-tauri/src/spectrum.rs), from
File > Open, a drop or a pasted path, as for JCAMP-DX.

## Mass Spec Analysis

A measured mass spectrum in the drawing is Mass Spec Analysis's peak list, so its ions are matched against it with no
paste: each peak above the spectrum's threshold (each stick of a peak table), its height against the tallest as 100.
A pasted list still takes its place. ChemDraw 26.1 matches nothing against measured data: opening a JCAMP mass
spectrum there says mass spectra are not supported.

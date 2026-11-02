# 0012: Structures in perspective (3D) as ChemDraw draws them

Status: built 21 October 2026, after Sam's report that he could not draw dirhodium paddlewheel carbene complexes
in perspective. Replaces how Octet drew 3D structures since Structure Perspective came in (16 October 2026).

## What was wrong

- A structure with depth (a turned one, a 3D Clean Up, or a ChemDraw file with `xyz`) was drawn by its own code:
  every bond as plain lines, thin at the back and bold at the front, faded with depth. Dashed, wedged, hashed,
  dative and partial double bonds lost their style, and labels went grey. ChemDraw does none of this.
- Octet read ChemDraw's `xyz` depth with the wrong sign, so the front and the back of every ChemDraw 3D drawing
  were swapped.
- Atoms drawn onto a turned structure were put at depth 0, so the next turn pulled them out of place.
- Part of a structure joined by more than one bond (a carboxylate bridging the Rh–Rh bond) could not turn on its
  own.

## Decision

Octet follows ChemDraw 26.1 (help and Sam's files; notes in `~/octet-research/notes/
perspective-and-crossing.md`):

- A 3D structure is drawn as a flat one is, at its atoms' x and y (no vanishing point), with every bond style and
  the document's widths. Its depth only decides crossings and depth shading.
- Where two bonds cross, the bond whose middle is nearer is in front. At the same depth (every flat drawing), the
  bond drawn later is in front, so Bring to Front and the f key work there only. The bond behind loses a section
  round the front bond: the front bond's own width (a single bond's line, a bold bond's bold width, a wedge from the
  line width to 1.5 bold widths, a double or triple bond's distance between its outer lines) and the Margin Width
  all round. A bond whose end only nearly meets another is cut too. The cut is part of the drawing, so every export
  has it.
- Depth shading is ChemDraw's Structure > Structure Perspective (⌥⌘S): off by default, on after 3D Clean Up, and
  stored as `ShowPerspective` in CDXML. It fades objects from full ink at the front of the drawing's depth range to a
  quarter at the back, and only when that range is more than 5 pt. Widths never change.
- CDXML `xyz` z points away from the viewer; Octet's z points towards it. Octet writes ChemDraw's sign and marks its
  files (`"xyz": 1` in the document's `Octet` data) so older Octet files keep their own.
- A turn follows ChemDraw's rate: 1.146° for each point the pointer moves on the page. Turning keeps depth; a drawing
  with depth opens with Select as Structure Perspective.
- Atoms drawn onto a turned structure take the depth of the atoms they are drawn from (ChemDraw gives a new atom
  its parent's depth). Flatten drops depth and leaves depth shading set.

Octet adds three things ChemDraw lacks:

- Part of a structure whose bonds to the rest all go to the same two atoms turns about the line through them
  (ChemDraw turns a part only about a single joining bond). Shift turns it in steps of 15°.
- The pointer takes the atom or bond in front where they lie over each other.
- The "very close to a bond" warning and the Clean Up suggestion count depth, so a structure in perspective is not
  marked for its crossing bonds.

## Not done

- 3D Clean Up's smaller labels (90 %) and margins (30 %) in ACS 1996 documents: Octet has no per-object margin.
- `CrossingBonds` in CDXML: ChemDraw writes it but never reads it, so Octet neither reads nor writes it.
- Six test files to draw in ChemDraw 26.1 to confirm these facts are listed in the research notes.

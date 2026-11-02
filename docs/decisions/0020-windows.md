# 0020: A window for each drawing

Status: built 2 November 2026, as Sam asked.

Octet had one window. Home covered the drawing, and opening a drawing replaced the one that was open. Sam asked for
more than one drawing open at a time, each in a window of its own, with Home listing the open windows.

## What others do

- ChemDraw 26.1 has a window for each document. New Document and Open Document open a new window. On Windows its
  toolbar has Cascade, Tile Horizontal and Tile Vertical (`~/octet-research/notes/chemdraw26-inventory.md`).
- Mac document apps (TextEdit, Pages) open a file into an untouched empty Untitled window instead of a new one.

## Decision

- Home is the window `main`. Each drawing has a window `draw-1`, `draw-2` and on (`src-tauri/src/windows.rs`).
- What opens from Home opens in a new window, and Home stays. New and Open in a drawing window open a new window,
  unless the window holds an untouched new drawing.
- Home has an Open windows section first, with each window's name, picture and state (Edited, Saved, New). A click
  brings the window to the front. A drawing that is open already comes to the front instead of opening twice.
- File > Home in a drawing window brings Home to the front, and opens it again if it was closed.
- Menu commands go to the window in front. Each window sends its ticks, colours and fonts when it comes to the front;
  the menu bar is built again only when a list changes. On the Mac the Window menu lists the windows.
- Quit asks about each window with unsaved changes in turn; Cancel stops it. The Dock's Quit and a logout do the same.
- Recovery keeps a copy for each window (`<session>z<window>`). Settings are shared through storage events.
- Files from Finder open one window each. A file dropped on a drawing goes into that drawing.

## Not done

Cascade and Tile (ChemDraw's Windows toolbar).

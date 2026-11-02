# 0015: Name to Structure reads nearly anything a chemist types

Status: built 27 October 2026 for Sam to review. It changes one rule of decision 0008: Octet itself now makes a
network connection for Name to Structure, as the updater does (decision 0010). It does so only with Settings >
Online search on, and only when the user presses Return on text nothing offline reads.

## ChemDraw 26.1

From the 26.1 help and testing, ChemDraw's Convert Name to Structure reads:

- systematic names, many German names, "alpha" and "beta" forms of amino acids, and some trade names;
- its list of FDA-approved drugs (973 names), then the user's own dictionary (Structure > Add Structure to
  Dictionary);
- peptide sequences (the library has peptide-sequence rules);
- grouped mixtures with ratios ("A--B (0.72/0.28)");
- online, since 25.8, ChemACX's synonyms. A name with more than one ChemACX structure is refused with a message.
  CAS numbers only through the ChemACX add-in, online.

A name it cannot read gives a message. Nothing in it suggests a spelling. Not known until a run of 26.1: whether
the Insert Structure sheet reads SMILES, InChI, a CAS number or several names at once
(`~/octet-research/notes/chemdraw-checks-pending.md`, item 13).

## What Octet reads, in order

1. Octet's own tables (`src/common.ts`, `src/drugs.ts`).
2. Names OPSIN misreads, from the dictionary (see below).
3. OPSIN: systematic names, many trivial names, CAS index names ("Benzoic acid, 2-hydroxy-").
4. The dictionary: common, trivial, German and trade names, CAS numbers (the check digit is checked; a wrong one shows
   "Not a CAS number. The last digit must be 5." under the field) and InChIKeys.
5. Condensed formulas: CH3CH2OH, PhCH2OH, Me3SiCl, PhB(OH)2, (CH3)3COH, CH3COOEt, HC≡CCH2OH, BnNH2. The reading
   uses the units of an atom label (`src/labels.ts`) and must use every valence once, so a molecular formula
   (C6H6, C2H6O) is refused: more than one structure has it.
6. Peptide sequences (Ala-Gly-Ser, H-Ala-D-Phe-OH, Ac-Ala-Gly-NH2; each residue L unless D- comes first) and
   "alpha alanine", "beta alanine", through their systematic names.
7. Salts, hydrates and mixtures, as before, and lists of names: commas, semicolons or new lines between. Each
   structure goes in a row, in the order typed.
8. SMILES and InChI, by their look (Indigo).
9. Online, after Return only: PubChem.

A name Octet cannot read gets a suggestion under the field, from OPSIN's names, Octet's tables and the dictionary
(a trigram index): "acetaminophin" gives acetaminophen, "benzaldehide" benzaldehyde.

The template library's search and the command palette find the dictionary's names after Octet's own
(`namesStarting`): "tylen" gives Tylenol, with its tile and Draw entry. Whatever the name field reads (a CAS number,
a condensed formula, a list) they read too.

## The dictionary

`data/names/dictionary.tsv.gz`, built by `scripts/build-dictionary.ts` from Wikidata (CC0) on 27 October 2026; see
`data/names/README.md`. The compounds are those with a SMILES and an article in one of ten large Wikipedias: the
compounds chemists name, not all of Wikidata's million. Each gives its English and German labels and aliases, the
names of brands with it as their one active ingredient, its CAS numbers and its InChIKey.

PubChem referees the names:

- A name OPSIN reads as the same compound needs no entry. Its label stays, for suggestions only.
- A name OPSIN reads as a compound with other bonds than Wikidata's goes to PubChem. Where PubChem has Wikidata's
  compound, the dictionary reads the name before OPSIN ("tropicamide", which OPSIN read as an amide of tropic acid).
  Never for a compound with a metal: OPSIN draws those as Octet does (sec-butyllithium bonded, not as ions).
  Where OPSIN differs only in stereo or charge, OPSIN keeps the name: it reads what the name says.
- A name easily meant as something else (a code of up to eight capitals and digits, any name of up to five
  characters, an English word or its plural) stays only where PubChem gives the same compound for it.
- A name or CAS number two compounds share stays with the one PubChem gives for it, or with neither.

The table has 28,880 compounds, 67,602 names (2,208 read before OPSIN) and 27,031 CAS numbers: 1.63 MB gzipped.
Of 200 random names PubChem knows, 181 give PubChem's structure exactly, 9 the same compound with other stereo, 6 the
same compound as a salt or its parent, and 4 another tautomer. Details in `data/names/README.md`.

## PubChem online

- When: Settings > Online search on, and the user presses Return (or ⇧⌘N) in the name field on text that nothing
  offline reads. Never while typing.
- What goes: the typed text (a name or an InChIKey) and nothing else, to PubChem's PUG REST service at the U.S.
  National Library of Medicine. A name goes in the request body, so no character of it can change the address.
- How: `src-tauri/src/pubchem.rs`, with the updater's HTTPS client (reqwest and rustls, nothing new in the build).
  Fixed address, 10 s limit, at most 64 KB back. Every line of the answer must be SMILES. More than one structure
  is refused with a message, as ChemDraw refuses more than one ChemACX structure. The page reads the SMILES with
  its usual limits and never passes it to `eval` or `innerHTML`.
- What the user sees: "Searching PubChem" under the field, then the structure and the notice "Drawn from PubChem."
  No answer or no network: the field shakes, as for any text Octet cannot read. The name and its structure join
  the names drawn before, so they draw again offline.

## Speed

- The dictionary loads with OPSIN, in slices: about 0.3 s in all, and no step blocks the window for more than about
  17 ms (measured in Node).
- A lookup takes under 1 µs. A suggestion as typing makes it takes 1.4 ms on average and under 4 ms at most, OPSIN's
  try included (`src/dictionary.check.ts` measures both and fails over 50 µs and 5 ms).

## Open questions

- The user's own dictionary (ChemDraw's Add Structure to Dictionary) is not built. Octet's names drawn before
  come back by name, but Octet cannot add a name to a drawn structure.
- One-letter peptide sequences ("AGS") are not read: they look like abbreviations.
- A trade name for a salt (Prozac is fluoxetine hydrochloride in PubChem) gives the parent when Wikidata gives it.

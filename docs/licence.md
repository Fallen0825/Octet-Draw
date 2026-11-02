# Licence plan

Status: decided, 14 October 2026. This is a plan, not legal advice.

## Decision

Octet is free and open-source software under the
[Apache License 2.0](../LICENSE).

- Anyone (students, universities, companies) can use, change and share
  Octet, and build it into other products.
- Every contributor gives a patent licence for their contributions, which
  protects users.
- The main chemistry libraries use the same family of licences (RDKit,
  Ketcher, Indigo, OPSIN), so Octet can use them and they can use Octet.

Accepted trade-off: someone can make a closed version of Octet.

This replaces the earlier plan for paid institutional licences.

## Contributions

Apache-2.0 section 5 licenses each contribution under Apache-2.0 unless
the contributor states otherwise, so no separate contributor agreement is
needed. If the project gets outside contributors, consider asking for a
Developer Certificate of Origin sign-off (`git commit -s`).

## Third-party libraries

Every library in Octet must have a licence that is compatible with
Apache-2.0.

- **Allowed for fonts only:** SIL Open Font License 1.1 (OFL-1.1). Keep
  each font's licence file with the app.
- **Allowed for data only:** Unicode License v3 (Unicode-3.0), for
  Unicode character data inside a dependency. Ship its notice with the
  app. Sam approved this on 16 October 2026.
- **Allowed:** MIT, BSD-2-Clause, BSD-3-Clause, Apache-2.0, ISC. For
  example: RDKit (BSD-3-Clause), OPSIN (MIT), Ketcher (Apache-2.0),
  Indigo (Apache-2.0), OpenChemLib (BSD-3-Clause).
- **Ask first:** LGPL, MPL-2.0, EPL and any other licence.
- **Not allowed:** GPL and AGPL (for example Open Babel), licences that
  forbid commercial use, and code with no licence.

Ship each library's licence notice with Octet.

Each dependency and its licence is listed in [THIRD_PARTY.md](../THIRD_PARTY.md).

## Using ChemDraw as the baseline

Octet takes ChemDraw's functionality as the minimum and builds on it, with
its own look and feel. Nobody owns chemical drawings, and functionality,
keyboard shortcuts and data file formats are not protected by copyright
(SAS Institute v World Programming, Court of Justice of the EU, 2012). The
repository is public, so ChemDraw's own code and assets stay out of it.

- Study ChemDraw only with your own licensed copy.
- Match its behaviour, workflow logic and keyboard shortcuts freely. Learn
  them by using and testing ChemDraw and from its public documentation.
- Never copy ChemDraw code, icons, templates, documentation text, sample
  files or data. Write our own code.
- Never touch licence or activation code.
- Use only files you created yourself as test files.
- Do not use "ChemDraw" in Octet's name or logo. Every comparison in
  marketing must be true.

## Open decisions

- [ ] **Name check.** Make sure "Octet" is free to use as a software
      name before launch.
- [ ] **Funding (optional):** donations, grants, sponsorship, or a paid
      hosted version.
- [x] **Tech stack.** Decided 14 October 2026: Tauri 2, React,
      TypeScript, Vite and Indigo. See `CLAUDE.md`.

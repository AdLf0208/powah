# Licence notice — Powah

- **Upstream:** Powah (Powah! Rearchitected), https://github.com/Technici4n/Powah,
  version 6.2.10 — the exact ATM10 NeoForge 1.21.1 jar `Powah-6.2.10.jar`
  (provenance: `ports/_shared/carry-work/powah/SOURCE.md`).
- **Licence as read:** **LGPL-3.0-only** — the upstream repository declares it and
  the port's [`LICENSE`](LICENSE) is the full GNU LGPL v3 text, keeping the
  original copyright notice beside the port's own.
- **What travels with the artifact:** this notice and [`LICENSE`](LICENSE) sit at
  the root of the distributed `powah.hmod` (the SDK package packs its root
  documents), so an artifact-only download carries them. The port is a
  **clean-room rewrite**: no Java was copied or translated and no jar was
  decompiled; the complete corresponding source of the Rust half is `src/` in the
  same package, and the full repository adds `notes.md`, `FINDINGS.md` and
  `CONTENT.md`.
- **Status:** local-only until the maintainer approves publishing
  (ARCHITECTURE.md §8). Under the LGPL, a published port is redistributed under
  the same licence with this complete source.

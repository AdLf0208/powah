# Licence notice — Powah

This port carries the original mod's assets and data and rewrites its behaviour
in Rust. The original licence and copyright notices are reproduced verbatim in
[LICENSE](LICENSE).

## Where the licence was read

| | |
|---|---|
| Pack jar | `Powah-6.2.10.jar` (2,737,991 B, SHA-256 `0e604a7356111c1dd44a00ea42fc1aa960d9faeb978261349df1138fcee4d0b4`) |
| Declared licence | `LGPLv3` in `META-INF/neoforge.mods.toml` |
| Licence text | the jar's own `LICENSE_Powah` (7,654 B, the full LGPL-3.0 text) (7,654 B) |
| SPDX identifier | `LGPL-3.0-only` |
| Copyright line | `Copyright (c) owmii, Technici4n, shartte and Powah contributors` |

The jar also ships `LICENSE_Powah`. The jar names the author(s) `owmii, Technici4n, shartte` but
carries no separate project copyright line, so the copyright line above uses
that author list; replace it with the upstream `LICENSE` copyright statement if
one is added later.

## The port

- Original: Powah 6.2.10 by
  owmii, Technici4n, shartte — LGPL-3.0-only
  (https://github.com/Technici4n/Powah).
- Port to Horizon by AdLf0208: the Rust runtime, the Horizon declarations and the
  build/verification tooling in this repository.
- This repository is the complete corresponding source: the licence requires
  that it travels with every copy and with every modified version.
- No Mojang file is included. The carried files are the mod's own assets and
  data; `data/minecraft/tags/…` files only add this mod's ids to vanilla tags.

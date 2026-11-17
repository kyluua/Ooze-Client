# L1 upstream content and feature-source review

Owner: Sol. Checked 2026-11-16 (America/Los_Angeles). Product matrix: docs/MODULES.md,
owned by Claude. This file records backend provenance and limits, not a complete
implementation of the referenced clients' modules.

## Feature references

- [Bunny modules](https://bunnyclient.lol/modules): the public text page reports
  121 modules/20 effects, but its dynamic list was not available in the text
  extraction. No complete list or source-code review is claimed.
- [Lunar features](https://www.lunarclient.com/features): confirms informational/
  convenience examples such as keystrokes, coordinates, freelook and toggle sprint.
  Marketing FPS multipliers are not Ooze performance evidence.
- [Badlion catalog](https://www.badlion.net/minecraft-client): confirms examples
  such as armor status, CPS, coordinates, crosshair, FPS, ping and zoom. A catalog
  listing is not a license to copy proprietary code/assets or proof of server policy.

Ooze keeps its own visuals and implementations. Research is not binary extraction.
Native UI product models are the project's own Rust code. No competing client
source or assets were copied into the launcher backend.

## Actual Modrinth installation fixture

Public API v2 requests use a descriptive User-Agent and no account token.
[Iris project](https://api.modrinth.com/v2/project/iris) ID YL57xq9U,
version ka9PDlsN (`1.10.8+1.21.11-fabric`), requires Sodium version rkdTcxoT,
project AANobbMI. This exact version pair is a Custom-instance integration test,
not a managed-policy approval or bundled release component. SHA-512 and sizes
come from the selected version metadata and are verified before commit.

Observed project license metadata:

| Project | ID | Reported license | Source |
|---|---|---|---|
| Iris | YL57xq9U | LGPL-3.0-only | https://github.com/IrisShaders/Iris |
| Sodium | AANobbMI | LicenseRef-Polyform-Shield-1.0.0 | https://github.com/CaffeineMC/sodium |
| Fabric API | P7dR8mSH | Apache-2.0 | https://github.com/FabricMC/fabric-api |

These are observations of each project's current API metadata, not a license
compatibility opinion. Sodium is not relabeled MIT or assumed to have the same
license as Iris. Its source and binaries have not been copied into Ooze's source
or launcher package by this work. User-selected downloads retain upstream terms.
Any future vendoring/bundling requires review of the exact version's license and
applicable notices. Managed currently approves only the pinned Ooze baseline.

The release's own notices are generated from the Windows-filtered Cargo metadata
and shipped license files; original font/OFL files are included. Minecraft game
files and these optional Custom mod downloads are not bundled in the review ZIP.

# Ooze 0.1.0-rc1

This candidate targets Windows x64. It ships an independent Rust launcher and
the Ooze base mod for Minecraft Java Edition 1.21.11 with Fabric Loader 0.20.1,
Fabric API 0.141.6+1.21.11 and a bundled Windows Rust engine. The launcher
installs its pinned Java 21 runtime and official Minecraft game files.

The candidate includes Microsoft browser sign-in through the Ooze application,
instances with verified installation and repair, Modrinth content with
dependency review, appearance settings and redacted diagnostics.
The base mod reports the engine state when the title opens. It leaves vanilla
usable if native loading fails.

In-game modules, additional loaders or versions and additional native
platforms are outside RC1. The separate Prism modpack installs only the Ooze
base mod and its required Fabric API. Prism supplies authentication and Play
for that instance.

This is an unsigned candidate. Verify SHA256SUMS and check the exact package
hash and acceptance status in the committed handoffs before distributing it.
See the [Sol RC1 handoff](https://github.com/kyluua/Ooze-Client/blob/main/docs/handoffs/SOL-RC1.md)
and [Claude RC1 handoff](https://github.com/kyluua/Ooze-Client/blob/main/docs/handoffs/CLAUDE-RC1.md).

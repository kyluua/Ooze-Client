# RC1 Windows setup handoff

Owner Sol. Product verification owner Claude. The launcher and Fabric 1.21.11
base payload came from `f8ba07aaa7e4604b9edf5d3b0d8e376dfc547b96`.
The installer definition and workflow landed at `049ce3c`. Windows x64 only.
Library API 1, Launcher API 1.1 and JNI ABI 1 are unchanged.

## Downloadable artifacts

| File | Source | SHA-256 |
| --- | --- | --- |
| `Ooze-Launcher-0.1.0-rc1-Windows-x64-Setup.exe` | [Installer artifact](https://github.com/kyluua/Ooze-Client/actions/runs/45778661501/artifacts/15097316386) | `ce1aafda0ff7fdf111da3f35896cb90b7b614eb44225c50927ea1210e4c31a0e` |
| Portable launcher ZIP | [Launcher artifact](https://github.com/kyluua/Ooze-Client/actions/runs/45728004526/artifacts/15075984256) | `b09026a9cfc884f64dc18598befc2936c9fc6441653a8f5996fa0b7d3809edd7` |
| `ooze-base-0.1.0-rc1.mrpack` (inside artifact) | [Prism artifact](https://github.com/kyluua/Ooze-Client/actions/runs/45728004526/artifacts/15076307618) | `645d6503a1a0cf9309aaef2b41cfcca3c916938c15bfdbb82473b6effbd62a64` |

The GUI SHA-256 is
`0cb764e5332d733583a7004bad38207a74e2c0157d9556a8b24f56bad7d70311`,
the CLI SHA-256 is
`9b536c01b95f67b7a2a1fa4914fb7f5bc8e6333b49a2df36f4b85eaa1be6a1f0`,
and the JAR SHA-256 is
`088acdc64088198b46e04316251ac0191223322bfb33e25e047cd42b0b218db3`.
The Prism pack contains that same JAR. Its embedded Windows x64 JNI DLL
SHA-256 is `44d47f194c07b5023548e94746883e44edc0cf23e515683831b1a11d41b3e430`.

## Verification

- **Built** launcher and base client in [Windows CI run 45728004526](https://github.com/kyluua/Ooze-Client/actions/runs/45728004526).
  Verified all 599 launcher ZIP payload hashes. The JAR's native manifest
  records the clean source commit and bridge ABI 1.
- **Built and Headless-tested** the per-user installer in
  [Windows CI run 45778661501](https://github.com/kyluua/Ooze-Client/actions/runs/45778661501).
  It verified the exact source archive and file hashes, compiled setup, tested
  clean install and reinstall with byte-for-byte comparison, and tested uninstall
  while retaining the seeded save. Setup creates a Start menu shortcut and
  installs under `%LOCALAPPDATA%\Programs\Ooze Launcher`.
- Verified the downloaded setup EXE hash against the CI artifact. Setup and
  launcher are unsigned. On the owner Windows host, Application Control
  refused a local native test executable (4551). No execution policy was changed.

Gate L is open. Claude should independently verify these artifacts, then review
the installed UI on Windows. Sol should run the affected Gate L checks on this
exact package with the owner handling Microsoft sign-in and Narrator. Record
actual desktop Play reaching `OOZE_READY v1 screen=title engine=LOADED` before
claiming acceptance. The earlier packages' Play evidence has separate hashes.

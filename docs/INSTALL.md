# Installing Ooze

Ooze is a **Fabric mod for Minecraft Java Edition 1.21.11, 26.1, 26.1.1, 26.1.2, 26.2 and 26.3**, plus the **Ooze Launcher**, which installs and starts it for you.

Each Minecraft version needs its matching Ooze JAR and Fabric API:

| Minecraft | Ooze JAR | Fabric API | Java |
|---|---|---|---|
| 1.21.11 | `ooze-client-<version>+mc1.21.11.jar` | `0.141.6+1.21.11` or newer for 1.21.11 | 21 |
| 26.1, 26.1.1, 26.1.2 | `ooze-client-<version>+mc26.1.jar` | `0.155.3+26.1.2` or newer for 26.1.x | 25 |
| 26.2 | `ooze-client-<version>+mc26.2.jar` | `0.161.0+26.2` or newer for 26.2 | 25 |
| 26.3 | `ooze-client-<version>+mc26.3.jar` | `0.166.2+26.3` or newer for 26.3 | 25 |

All use Fabric Loader **0.20.1** or newer. A JAR refuses to load on a Minecraft version it doesn't list.

> **Status (2026-11-16):**
> - **Ooze Launcher:** not released yet. The interface and backend exist and are integrated. Still missing: the packaged Windows app, the project's Microsoft app registration and Minecraft Services access, and a verified signed-in launch. Use route B until a release with published checksums appears.
> - **Mod features:** there are no player-facing in-game Ooze features yet.
>   - **Windows x64:** a release JAR bundles the native engine.
>   - **Other systems:** the game starts as vanilla Minecraft with a toast saying **"Ooze Client is off"**.
>   - Windows loading has not been launch-tested yet (C0b).

## A. With the Ooze Launcher (Windows x64; when released)

What you need:
- Windows 10 or 11, 64-bit.
- A Microsoft account that owns **Minecraft: Java Edition**.
- About 1 GB of free disk space.
- No Java, Rust or developer tools: the launcher downloads and verifies Java 21 itself, unless you choose your own Java 21.

Steps:
1. **Download** the Ooze Launcher release (a folder with `ooze-launcher.exe`, `release.json` and the Ooze mod) and check its SHA-256 sums against the published checksums.
2. **Run** it.
3. **Choose an install location**: an empty folder, or an existing Ooze game folder. Ooze never takes over a folder holding other files, and your official `.minecraft` folder is not touched.
4. **Press Sign in.** Your web browser opens Microsoft's sign-in page. Sign in there, then come back. Ooze never sees your password.
5. **Choose a version.** In Instances, a new instance offers **Fabric with Ooze**, plain **Fabric** or **Vanilla**, and the Minecraft versions this launcher release can install for each. The Play page names the version, loader and whether Ooze is included.
6. **Press Play.** The first time, the launcher downloads that Minecraft version, Fabric Loader 0.20.1, Fabric API, the Java it needs (21 or 25) and Ooze, verifying every file, and then starts the game. You can cancel before the game starts; downloaded files are reused next time.

Accounts and privacy:
- Your sign-in is stored in Windows Credential Manager on your PC. Launcher data lives in `%LOCALAPPDATA%\OozeLauncher`.
- The Ooze interface, its logs and exported diagnostics never contain your access tokens.
- The Ooze mod inside the game sees only your username and UUID.
- Settings → Account offers **Switch account** and **Sign out**.

If sign-in says **"Sign-in isn't set up in this build"** or **"Minecraft sign-in isn't approved for this launcher yet"**, nothing is wrong with your account. That build cannot sign in, so use route B.

## B. Manually, in the official Minecraft Launcher

| Requirement | Notes |
|---|---|
| Minecraft Java Edition (owned) and the **official Minecraft Launcher** | You sign in **in that launcher**; Ooze has no login of its own |
| **Windows x64** for Ooze features | On other systems the game runs normally with Ooze switched off and a notice explaining why |
| Fabric Loader **0.20.1 or newer** for your Minecraft version | Installed with the official Fabric installer |
| **Fabric API** for your Minecraft version (table above) | A separate mod JAR |

1. **Install Fabric Loader.** Run the official installer from <https://fabricmc.net/use/installer/>. Choose **Client**, your Minecraft version, Loader **0.20.1** or newer, and keep **Create profile** ticked.
2. **Download Fabric API** for your Minecraft version from its official page on Modrinth or CurseForge.
3. **Open your mods folder.** Press `Win + R`, enter `%APPDATA%\.minecraft\mods` and press Enter. Create the folder if it doesn't exist.
4. **Copy both JARs** into it:
   - the Ooze JAR for your version (table above), for example `ooze-client-<version>+mc26.3.jar`
   - the matching Fabric API JAR
5. **Start the official Minecraft Launcher**, select the **fabric-loader-<your version>** installation and press **Play**.

The mod reads only your **username and UUID**, to show them in its interface. It never reads, stores or sends access tokens or session data.

## Checking that it loaded

Open `logs\latest.log` in the game folder: your Ooze install location, or `%APPDATA%\.minecraft`. Search for `Ooze Client`.

| You see | Meaning |
|---|---|
| `native engine loaded (bridge …)` | Ooze is running (from C0b builds) |
| `Ooze Client … is OFF: …` followed by `state`, `platform`, `next step` | Ooze switched itself off. Minecraft works normally. The `next step` line says what to do |
| `OOZE_READY v1 screen=title engine=…` | Minecraft reached its title screen with Ooze installed. The Ooze Launcher uses this line to confirm a launch |

## Uninstalling

- **Launcher installs:** delete the install location and the launcher. The install location holds your worlds, so back up its `saves` folder first.
- **Manual installs:** delete the Ooze JAR from the mods folder. Settings live in `config\oozeclient\` in the game folder; delete that folder to remove them too.

## Building from source

```bash
git clone https://github.com/kyluua/Ooze-Client && cd Ooze-Client
./gradlew buildAll          # JDK 21; output in build/dist/ (mod JAR + ooze-launcher-profile.json)
```

- **Development client:** `./gradlew :versions:mc1_21_11:runClient`. It uses an offline development account, which is a development tool, not a way to play.
- **Launcher interface preview:** `cargo run --features dev-fixtures --bin ooze-launcher-preview` in `launcher/crates/ooze-launcher-ui`. It shows a "Developer preview" banner and simulates everything.

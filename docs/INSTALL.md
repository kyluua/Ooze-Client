# Installing Ooze Client

Ooze Client is a **Fabric mod for Minecraft Java Edition 1.21.11**. You play it through the **official Minecraft Launcher**, exactly like any other Fabric mod.

> **Current build (C0):** there are no player-facing Ooze features yet; they start in later milestones.
> - **Windows x64:** a release JAR bundles the native engine, which loads silently.
> - **Other systems:** the game starts as normal vanilla Minecraft and shows a toast saying **"Ooze Client is off"**.
> - A JAR built without the native resources shows the same toast on Windows.
> - Windows loading has not been launch-tested yet (C0b).

## What you need

| Requirement | Notes |
|---|---|
| Minecraft Java Edition (owned) and the **official Minecraft Launcher** | You sign in to your Microsoft account **in the launcher**. Ooze never asks for your account and has no login of its own |
| **Windows x64** for Ooze features | On other systems the game runs normally with Ooze switched off and a notice explaining why |
| Fabric Loader **0.20.1 or newer** for 1.21.11 | Installed with the official Fabric installer |
| **Fabric API 0.141.6+1.21.11** (or newer for 1.21.11) | A separate mod JAR |

## Steps (Windows)

1. **Install Fabric Loader.** Run the official installer from <https://fabricmc.net/use/installer/>. Choose **Client**, Minecraft **1.21.11**, Loader **0.20.1** or newer, and keep **Create profile** ticked.
2. **Download Fabric API** for 1.21.11 from its official page on Modrinth or CurseForge.
3. **Open your mods folder.** Press `Win + R`, enter `%APPDATA%\.minecraft\mods` and press Enter. Create the folder if it doesn't exist.
4. **Copy both JARs** into it:
   - `ooze-client-<version>+mc1.21.11.jar`
   - `fabric-api-<version>+1.21.11.jar`
5. **Start the official Minecraft Launcher**, select the **fabric-loader-1.21.11** installation and press **Play**.

## Accounts and privacy

- Sign-in, ownership checks and the game session are handled by the launcher and Minecraft.
- Ooze reads only your **username and UUID**, to show them in its interface. It never reads, stores or sends access tokens or session data.
- To switch accounts, use the launcher's account menu. Ooze has no account switcher.

## Checking that it loaded

Open `%APPDATA%\.minecraft\logs\latest.log` and search for `Ooze Client`.

| You see | Meaning |
|---|---|
| `native engine loaded (bridge …)` | Ooze is running (from C0b builds) |
| `Ooze Client … is OFF: …` followed by `state`, `platform`, `next step` | Ooze switched itself off. Minecraft works normally. The `next step` line says what to do |

## Uninstalling

Delete the Ooze JAR from the mods folder. Settings live in `%APPDATA%\.minecraft\config\oozeclient\`; delete that folder to remove them too.

## Building from source

```bash
git clone https://github.com/kyluua/Ooze-Client && cd Ooze-Client
./gradlew buildAll          # JDK 21; output in build/dist/
```

The development client uses an offline development account (`./gradlew :versions:mc1_21_11:runClient`). That is a development tool, not a way to play.

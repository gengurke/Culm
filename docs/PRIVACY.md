# Privacy

Culm has no telemetry, no analytics and no accounts. It only talks to the services below, only to do what you
asked for, and only sends what that needs. Every request carries a generic user agent
(`addon-manager/<version> (World of Warcraft: Forever addon manager)`) and, like any web request, your IP address.

| Service | When | What Culm sends |
|---|---|---|
| **CurseForge** (`api.curseforge.com`) | Search, update checks, matching your installed addons, installs | Your search text, addon ids, and fingerprints (number hashes) of your addon folders. Only if you added an API key; the key goes to CurseForge's own hosts (`curseforge.com`, `forgecdn.net`) and nowhere else, also not after a redirect. |
| **WowUp Hub** (`hub.wowup.io`) | Search, featured addons, matching your installed addons, update checks | Your search text, addon ids, and fingerprints (hashes) of your addon folders. |
| **GitHub** (`api.github.com`, `github.com`) | Installing from a GitHub link, update checks for GitHub addons, the description of any addon that names a GitHub repository | Those repository names. |
| **GitHub** (`api.github.com`) | Culm's own update check: once per start for builds that can update themselves, or when you press the button in Settings → About | Nothing about you or your addons. |
| **Blizzard** (`render.worldofwarcraft.com`) | Until the class icons are downloaded once | Nothing about you. |
| Addon downloads and pictures | Installing or updating an addon, showing its thumbnail | The download and image addresses come from the sources above (e.g. CurseForge's or GitHub's download servers). |

Some CurseForge addons can only be downloaded through CurseForge's website: then Culm opens the page in your
browser and installs the file from your Downloads folder.

## What stays on your computer

Your installations, addon lists, settings (including the CurseForge API key) and backups are stored only on your
computer, in the folders listed in the README under "Where data lives". Pictures from CurseForge are kept only in
memory; other thumbnails are cached on disk. Culm reads your WoW folder and other addon managers' records (for the
import) but sends none of it anywhere, apart from the folder fingerprints above.

Culm writes the errors you see and its crashes to a small log file on your computer. Settings → About →
"Copy debug info" copies its last errors with Culm's version, your system and your game folders to the clipboard,
for you to paste into a bug report; your home folder is shortened to `~` and the CurseForge key is never included.
Nothing is sent by itself.

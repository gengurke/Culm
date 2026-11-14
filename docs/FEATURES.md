# Features

What the addon manager does, one short paragraph per feature. Ids are referenced by test names
(`f2_1_...`). Status: ✅ done · 🚧 partial · 📋 planned.

## F1 Installations
- **F1.1 Auto-detect** 📋 — Finds WoW Forever clients (`_forever_` folders whose `.build.info` row has
  product `wowf`) in common places: Wine/Proton/Faugus/Lutris/Bottles prefixes, `~/Games`, Windows default path.
  Reads the client version (e.g. `1.60.1.70938`).
- **F1.2 Add manually** 📋 — Pick any folder (WoW root, `_forever_`, or `Wow.exe`'s folder); it is
  normalized to the `_forever_` client folder and validated.
- **F1.3 Multiple installs** 📋 — Several installations can be stored; one is selected. Each has its own addon list.

## F2 My Addons (local)
- **F2.1 Scan** 📋 — Reads `Interface/AddOns`, picks the right TOC per folder (Forever: `_Forever`/`_Camelot`
  first, then the plain TOC), and shows title, version, author, interface version.
- **F2.2 Grouping** 📋 — Folders that belong to one addon (from provider data, or TOC dependencies such
  as `Details_*`) are shown as one entry.
- **F2.3 Matching** 📋 — Folders are matched to CurseForge via fingerprints (murmur2)
  and to other providers via TOC ids (`X-Curse-Project-ID`, `X-WoWI-ID`). Unmatched addons are shown as "Unknown" and ignored for updates.
- **F2.4 WowUp import** 📋 — Imports addon records from an existing WowUp installation for the same
  client, so switching from WowUp keeps provider links without rescanning online.
- **F2.5 Persistence** 📋 — The addon list per installation is stored as JSON in the app's data dir and
  survives restarts; a rescan refreshes it.

## F3 Updates
- **F3.1 Check for updates** 📋 — On start and on demand, fetches latest files for all managed addons
  (batch per provider) and marks addons with "Update available".
- **F3.2 Update / Update all** 📋 — Downloads the release zip, swaps the addon folders safely (backup,
  extract, copy, rollback on failure) and records the new version.
- **F3.3 Channels** 📋 — Per addon Stable / Beta / Alpha; "Beta" also accepts stable files, etc.
- **F3.4 Ignore** 📋 — Ignored addons are never updated; can be un-ignored. Git checkouts are ignored automatically.

## F4 Get Addons (remote)
- **F4.1 Search** 📋 — Search CurseForge for Forever addons; results show name, author, downloads, summary.
- **F4.2 Popular** 📋 — Without a query, shows popular/recently updated Forever addons.
- **F4.3 Install** 📋 — Installs a search result; installed addons are marked as such.
- **F4.4 Install from URL** 📋 — Paste a CurseForge addon URL or a GitHub repo URL (releases with
  `release.json` or a `*-forever.zip`/plain zip asset).

## F5 Manage
- **F5.1 Remove** 📋 — Deletes the addon's folders and its record.
- **F5.2 Reinstall** 📋 — Re-downloads the installed (or latest) release.
- **F5.3 Open links** 📋 — Open the addon page / the AddOns folder.

## F6 Maintenance
- **F6.1 Clear cache** 📋 — Deletes the client's `Cache` folder (or just `Cache/WDB`). Refused while WoW runs.
- **F6.2 Clean backups** 📋 — Deletes `WTF/**/*.lua.bak` and `WTF/*.old` leftovers.
- **F6.3 WTF backup** 📋 — Zips the `WTF` folder (settings + SavedVariables) into the app's data dir; lists backups.

## F7 Settings
- **F7.1 CurseForge API key** 📋 — Stored in prefs; without a key, CurseForge features are disabled with a hint.
- **F7.2 Defaults** 📋 — Default channel for new addons.

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
- **F2.3 Matching** 🚧 — Folders are matched to CurseForge via fingerprints (murmur2)
  and to other providers via TOC ids (`X-Curse-Project-ID`, `X-WoWI-ID`). Unmatched addons are shown as "Unknown" and ignored for updates.
  Provider side done: CurseForge `POST /v1/fingerprints/1` maps each matched file's `modules[].name` to folders and
  reports the installed file; WowInterface matches `("WowInterface", id)` TOC ids
  (`f2_3_fingerprint_match_maps_modules_to_folders`, `f2_3_wowinterface_matches_toc_ids`).
- **F2.4 WowUp import** 📋 — Imports addon records from an existing WowUp installation for the same
  client, so switching from WowUp keeps provider links without rescanning online.
- **F2.5 Persistence** 📋 — The addon list per installation is stored as JSON in the app's data dir and
  survives restarts; a rescan refreshes it.

## F3 Updates
- **F3.1 Check for updates** 🚧 — On start and on demand, fetches latest files for all managed addons
  (batch per provider) and marks addons with "Update available". Provider `get_many` done (CurseForge `POST /v1/mods`,
  GitHub per repo, WowInterface filedetails), newest Forever file per channel (`f3_1_get_many_reports_latest_file_per_channel`).
- **F3.2 Update / Update all** 📋 — Downloads the release zip, swaps the addon folders safely (backup,
  extract, copy, rollback on failure) and records the new version. `http::download` (with progress, clear HTTP
  errors) is ready (`f3_2_download_reports_progress_and_http_errors`).
- **F3.3 Channels** 📋 — Per addon Stable / Beta / Alpha; "Beta" also accepts stable files, etc.
- **F3.4 Ignore** 📋 — Ignored addons are never updated; can be un-ignored. Git checkouts are ignored automatically.

## F4 Get Addons (remote)
- **F4.1 Search** 🚧 — Search CurseForge for Forever addons; results show name, author, downloads, summary.
  Provider done: `gameVersionTypeId=88568`, sorted by popularity, files filtered to Forever (no alternatives), mods
  without a Forever file dropped (`f4_1_search_curseforge_filters_forever_files`).
- **F4.2 Popular** 🚧 — Without a query, shows popular/recently updated Forever addons. Provider done:
  `POST /v1/mods/featured`, popular + recentlyUpdated deduped (`f4_2_popular_lists_featured`).
- **F4.3 Install** 📋 — Installs a search result; installed addons are marked as such.
- **F4.4 Install from URL** 🚧 — Paste a CurseForge addon URL or a GitHub repo URL (releases with
  `release.json` or a `*-forever.zip`/plain zip asset). Provider `from_url` done for CurseForge (slug search),
  GitHub (`owner/repo`, newest 5 non-draft releases, prerelease = Beta; plain-zip fallback skips zips tagged for other
  flavors) and WowInterface (`info<ID>-...`) (`f4_4_curseforge_url_resolves_slug`, `f4_4_github_url_picks_forever_asset`,
  `f4_4_github_plain_zip_fallback_skips_other_flavors`).

## F5 Manage
- **F5.1 Remove** 📋 — Deletes the addon's folders and its record.
- **F5.2 Reinstall** 📋 — Re-downloads the installed (or latest) release.
- **F5.3 Open links** 📋 — Open the addon page / the AddOns folder.

## F6 Maintenance
- **F6.1 Clear cache** 📋 — Deletes the client's `Cache` folder (or just `Cache/WDB`). Refused while WoW runs.
- **F6.2 Clean backups** 📋 — Deletes `WTF/**/*.lua.bak` and `WTF/*.old` leftovers.
- **F6.3 WTF backup** 📋 — Zips the `WTF` folder (settings + SavedVariables) into the app's data dir; lists backups.

## F7 Settings
- **F7.1 CurseForge API key** 🚧 — Stored in prefs; without a key, CurseForge features are disabled with a hint.
  Provider side done: `available()` is false without a key and calls fail with an "API key" error before any request;
  `providers::all(&ProviderConfig)` builds the list (`f7_1_curseforge_unavailable_without_key`).
- **F7.2 Defaults** 📋 — Default channel for new addons.

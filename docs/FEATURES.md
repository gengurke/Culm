# Features

What the addon manager does, one short paragraph per feature. Ids are referenced by test names
(`f2_1_...`). Status: ✅ done · 🚧 partial · 📋 planned.

## F1 Installations
- **F1.1 Auto-detect** ✅ — Finds WoW Forever clients (`_forever_` folders whose `.build.info` row has
  product `wowf`) in Wine/Proton/Faugus/Lutris/Bottles prefixes and `~/Games` (one `read_dir` per parent, no disk walk).
  Reads the client version (e.g. `1.60.1.70938`). (`f1_1_detects_forever_install_from_build_info`)
- **F1.2 Add manually** ✅ — Pick any folder (WoW root, `_forever_`, `Interface/AddOns`, or `Wow.exe`); it is
  normalized to the `_forever_` client folder and validated. (`f1_2_normalizes_manual_paths`)
- **F1.3 Multiple installs** 🚧 — Several installations are stored in `installations.json` (id = hash of the path),
  each with its own addon list; selection is UI state, not done yet. (`f1_3_installations_persist`)

## F2 My Addons (local)
- **F2.1 Scan** ✅ — Reads `Interface/AddOns`, picks the right TOC per folder (Forever: `_Forever`/`_Camelot`
  first, then the plain TOC), and reads title, version, author, interface version. (`f2_1_scan_picks_forever_toc`)
- **F2.2 Grouping** ✅ — Folders that belong to one addon (same provider match, folders listed in the installed file,
  or unmatched folders requiring a prefix parent such as `Details_*`) form one entry. (`f2_2_groups_by_provider_and_dependencies`)
- **F2.3 Matching** ✅ — Folders are matched to CurseForge via fingerprints (murmur2)
  and to other providers via TOC ids (`X-Curse-Project-ID`, `X-WoWI-ID`). Unmatched addons are "Unknown" and ignored for updates;
  git checkouts are ignored as such. (`f2_3_fingerprint_matches_golden_value`, `f2_3_unmatched_are_unknown_and_provider_errors_are_reported`)
- **F2.4 WowUp import** ✅ — Imports addon records (`~/.config/WowUpCf`, `~/.config/WowUp`) of the WowUp installation in the same
  client folder, so switching from WowUp keeps provider links without rescanning online. (`f2_4_imports_wowup_records`)
- **F2.5 Persistence** ✅ — The addon list per installation is stored as JSON (`<data>/addons/<id>.json`, atomic write);
  a rescan refreshes it, keeping ids, channel, auto-update, user ignores and offline provider links. (`f2_5_store_roundtrip_and_rescan_keeps_user_choices`)

## F3 Updates
- **F3.1 Check for updates** 📋 — On start and on demand, fetches latest files for all managed addons
  (batch per provider) and marks addons with "Update available".
- **F3.2 Update / Update all** 🚧 — Downloads the release zip, validates it (no `..`/absolute paths/symlinks), extracts next to AddOns,
  swaps the folders with backup + rollback, removes folders the new version dropped, records the new version.
  Core done, UI pending. (`f3_2_update_swaps_folders_and_rolls_back_on_bad_zip`, `f3_2_swap_failure_restores_backups`)
- **F3.3 Channels** 📋 — Per addon Stable / Beta / Alpha; "Beta" also accepts stable files, etc.
- **F3.4 Ignore** 📋 — Ignored addons are never updated; can be un-ignored. Git checkouts are ignored automatically.

## F4 Get Addons (remote)
- **F4.1 Search** 📋 — Search CurseForge for Forever addons; results show name, author, downloads, summary.
- **F4.2 Popular** 📋 — Without a query, shows popular/recently updated Forever addons.
- **F4.3 Install** 🚧 — Installs a search result (core: `install_remote` + `upsert_addon`); UI pending. (`f4_3_install_creates_record_and_replaces_existing_folders`)
- **F4.4 Install from URL** 📋 — Paste a CurseForge addon URL or a GitHub repo URL (releases with
  `release.json` or a `*-forever.zip`/plain zip asset).

## F5 Manage
- **F5.1 Remove** 🚧 — Deletes the addon's folders (core done, UI pending). (`f5_1_remove_deletes_folders`)
- **F5.2 Reinstall** 🚧 — Re-downloads the given release over the addon (core done, UI pending). (`f5_2_reinstall_restores_modified_files`)
- **F5.3 Open links** 📋 — Open the addon page / the AddOns folder.

## F6 Maintenance
- **F6.1 Clear cache** 🚧 — Deletes the client's `Cache` folder (or just `Cache/WDB`). All F6 tools refuse while a `Wow.exe` of this
  client runs. Core done, UI pending. (`f6_1_clear_cache_removes_cache_dir`, `f6_1_refuses_while_game_running`)
- **F6.2 Clean backups** 🚧 — Deletes `WTF/**/*.lua.bak` and `WTF/*.old` leftovers (core done). (`f6_2_clean_backup_files`)
- **F6.3 WTF backup** 🚧 — Zips the `WTF` folder (settings + SavedVariables) into a backups dir; lists backups (core done). (`f6_3_backup_wtf_zips_and_lists`)

## F7 Settings
- **F7.1 CurseForge API key** 📋 — Stored in prefs; without a key, CurseForge features are disabled with a hint.
- **F7.2 Defaults** 📋 — Default channel for new addons.

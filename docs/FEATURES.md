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
  git checkouts are ignored as such. CurseForge `POST /v1/fingerprints/1` maps matched files' `modules[].name` to folders. (`f2_3_fingerprint_match_maps_modules_to_folders`, `f2_3_wowinterface_matches_toc_ids`, `f2_3_fingerprint_matches_golden_value`, `f2_3_unmatched_are_unknown_and_provider_errors_are_reported`)
- **F2.4 WowUp import** ✅ — Imports addon records (`~/.config/WowUpCf`, `~/.config/WowUp`) of the WowUp installation in the same
  client folder, so switching from WowUp keeps provider links without rescanning online. (`f2_4_imports_wowup_records`)
- **F2.5 Persistence** ✅ — The addon list per installation is stored as JSON (`<data>/addons/<id>.json`, atomic write);
  a rescan refreshes it, keeping ids, channel, auto-update, user ignores and offline provider links. (`f2_5_store_roundtrip_and_rescan_keeps_user_choices`)

## F3 Updates
- **F3.1 Check for updates** 🚧 — On start and on demand, fetches latest files for all managed addons
  (batch per provider) and marks addons with "Update available". Provider `get_many` done (CurseForge `POST /v1/mods`,
  GitHub per repo, WowInterface filedetails), newest Forever file per channel (`f3_1_get_many_reports_latest_file_per_channel`).
- **F3.2 Update / Update all** 🚧 — Downloads the release zip, validates it (no `..`/absolute paths/symlinks), extracts next to AddOns,
  swaps the folders with backup + rollback, removes folders the new version dropped, records the new version.
  Core done, UI pending. (`f3_2_download_reports_progress_and_http_errors`, `f3_2_update_swaps_folders_and_rolls_back_on_bad_zip`, `f3_2_swap_failure_restores_backups`)
- **F3.3 Channels** 📋 — Per addon Stable / Beta / Alpha; "Beta" also accepts stable files, etc.
- **F3.4 Ignore** 📋 — Ignored addons are never updated; can be un-ignored. Git checkouts are ignored automatically.

## F4 Get Addons (remote)
- **F4.1 Search** 🚧 — Search CurseForge for Forever addons; results show name, author, downloads, summary.
  Provider done: `gameVersionTypeId=88568`, sorted by popularity, files filtered to Forever (no alternatives), mods
  without a Forever file dropped (`f4_1_search_curseforge_filters_forever_files`).
- **F4.2 Popular** 🚧 — Without a query, shows popular/recently updated Forever addons. Provider done:
  `POST /v1/mods/featured`, popular + recentlyUpdated deduped (`f4_2_popular_lists_featured`).
- **F4.3 Install** 🚧 — Installs a search result (core: `install_remote` + `upsert_addon`); UI pending. (`f4_3_install_creates_record_and_replaces_existing_folders`)
- **F4.4 Install from URL** 🚧 — Paste a CurseForge addon URL or a GitHub repo URL (releases with
  `release.json` or a `*-forever.zip`/plain zip asset). Provider `from_url` done for CurseForge (slug search),
  GitHub (`owner/repo`, newest 5 non-draft releases, prerelease = Beta; plain-zip fallback skips zips tagged for other
  flavors) and WowInterface (`info<ID>-...`) (`f4_4_curseforge_url_resolves_slug`, `f4_4_github_url_picks_forever_asset`,
  `f4_4_github_plain_zip_fallback_skips_other_flavors`).

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
- **F7.1 CurseForge API key** 🚧 — Stored in prefs; without a key, CurseForge features are disabled with a hint.
  Provider side done: `available()` is false without a key and calls fail with an "API key" error before any request;
  `providers::all(&ProviderConfig)` builds the list (`f7_1_curseforge_unavailable_without_key`).
- **F7.2 Defaults** 📋 — Default channel for new addons.

## UI shell 🚧
Rune UI in `crates/app` (Moonlit look): sidebar nav with update badge and
installation picker, starry header band, My Addons (filter, virtual list, detail panel), Get Addons
(debounced search, popular list, install from URL, no-key hint), Maintenance (confirm + result per
card, locked while the game runs), Settings (API key, default channel, installations, WowUp import),
toasts and onboarding. It runs on `MockBackend` until the core is wired behind `backend::Backend`.
Runtime tests in `crates/app/tests/ui_shell.rs` drive the UI side of these features:
F1.2 `f1_2_no_installation_shows_onboarding` · F2.1 `f2_1_boot_scans_and_checks_updates`,
`f2_1_filter_narrows_the_list` · F3.2 `f3_2_update_button_updates_addon`, `f3_2_failed_update_shows_error`,
`f3_2_update_all_updates_everything_available` · F3.4 `f3_4_ignore_from_detail_panel` ·
F4.1/F4.3 `f4_1_search_shows_results_and_f4_3_install_marks_installed` · F4.2 `f4_2_popular_list_loads` ·
F5.1 `f5_1_remove_needs_confirm` · F5.3 `f5_3_open_website` · F6.1 `f6_1_clear_cache_confirm_flow`,
`f6_1_maintenance_locked_while_game_runs` · F6.3 `f6_3_wtf_backup_lists_new_backup` ·
F7.1 `f7_1_without_key_get_addons_points_to_settings` · plus `navigation_switches_screens`, `slash_focuses_the_filter`.

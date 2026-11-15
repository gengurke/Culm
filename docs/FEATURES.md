# Features

What the addon manager does, one short paragraph per feature. Ids are referenced by test names
(`f2_1_...`). Status: ✅ done · 🚧 partial · 📋 planned.

## F1 Installations
- **F1.1 Auto-detect** ✅ — Finds WoW Forever clients (`_forever_` folders whose `.build.info` row has
  product `wowf`) in Wine/Proton/Faugus/Lutris/Bottles prefixes and `~/Games` (one `read_dir` per parent, no disk walk).
  Reads the client version (e.g. `1.60.1.70938`). Detected clients are stored with the manual ones and the first
  is selected on start. (`f1_1_detects_forever_install_from_build_info`, e2e `f1_1_detected_installation_is_selected_and_scanned`)
- **F1.2 Add manually** ✅ — Pick any folder (WoW root, `_forever_`, `Interface/AddOns`, or `Wow.exe`); it is
  normalized to the `_forever_` client folder and validated; onboarding offers the folder picker when nothing was found.
  (`f1_2_normalizes_manual_paths`, e2e `f1_2_installation_added_from_folder_picker_is_remembered`)
- **F1.3 Multiple installs** ✅ — Several installations are stored in `installations.json` (id = hash of the path),
  each with its own addon list; the sidebar picker switches, and the choice is kept in prefs. (`f1_3_installations_persist`)

## F2 My Addons (local)
- **F2.1 Scan** ✅ — Reads `Interface/AddOns`, picks the right TOC per folder (Forever: `_Forever`/`_Camelot`
  first, then the plain TOC), and reads title, version, author, interface version. Rows show provider thumbnails
  (downloaded on a worker, cached in the cache dir) over an initials tile. (`f2_1_scan_picks_forever_toc`,
  e2e `f2_1_f2_3_scan_shows_matched_and_unknown_addons`, `f2_1_thumbnails_load_over_initials`)
- **F2.2 Grouping** ✅ — Folders that belong to one addon (same provider match, folders listed in the installed file,
  or unmatched folders requiring a prefix parent such as `Details_*`) form one entry. (`f2_2_groups_by_provider_and_dependencies`)
- **F2.3 Matching** ✅ — Folders are matched to CurseForge via fingerprints (murmur2)
  and to other providers via TOC ids (`X-Curse-Project-ID`; the WowInterface provider exists but is disabled for now because its files carry no game-version info). Unmatched addons are "Unknown" and ignored for updates;
  git checkouts are ignored as such. CurseForge `POST /v1/fingerprints/1` maps matched files' `modules[].name` to folders. (`f2_3_fingerprint_match_maps_modules_to_folders`, `f2_3_wowinterface_matches_toc_ids`, `f2_3_fingerprint_matches_golden_value`, `f2_3_unmatched_are_unknown_and_provider_errors_are_reported`)
- **F2.4 WowUp import** ✅ — Imports addon records (`~/.config/WowUpCf`, `~/.config/WowUp`) of the WowUp installation in the same
  client folder, so switching from WowUp keeps provider links without rescanning online. Runs automatically on the first
  scan of an installation (and from Settings). A folder belongs to one record: when WowUp lists it under two addons
  (Questie also lists QuestieTrace), the record with fewer folders keeps it.
  A WowUp link is not taken over by another provider's TOC-id match (WowInterface's file for Enhance QoL is not the
  Forever build). Versions stored as zip names (`ForeverQuestTint-0.7.3.zip`) show the TOC `## Version`.
  Only a newer release WowUp knew of is imported as `latest`; "WowUp's latest is what's installed" is not our check,
  so those addons show "Not checked" until a CurseForge key lets us check.
  (`f2_4_imports_wowup_records`, `f2_4_overlapping_wowup_records_get_one_owner_per_folder`,
  `f2_4_wowup_link_is_not_taken_over_by_another_providers_toc_match`, `f2_4_zip_file_name_versions_show_the_toc_version`,
  e2e `f2_4_first_scan_imports_wowup_records_without_a_key`)
- **F2.5 Persistence** ✅ — The addon list per installation is stored as JSON (`<data>/addons/<id>.json`, atomic write);
  a rescan refreshes it, keeping ids, channel, auto-update, user ignores and offline provider links. On a shared folder a
  fresh match of the same provider wins, a link to another provider stays with the stored record. Store writes are
  serialized in the app, fsynced, and a scan waits for running installs so it never sees a folder mid-swap.
  (`f2_5_store_roundtrip_and_rescan_keeps_user_choices`)

## F3 Updates
- **F3.1 Check for updates** ✅ — After each scan and on demand, `updates::check_updates` asks each available provider
  once for all managed (linked, not ignored) addons and marks "Update available"; a failing provider (or GitHub repo) is
  reported without stopping the rest, providers without a key are skipped. A managed addon no check has answered for
  (no `latest`, e.g. CurseForge without a key) shows a muted "Not checked", never "Up to date", and without a key the header says
  "CurseForge updates not checked — add a CurseForge key" (the imported last known releases are all it has). A CurseForge display name that is a zip name shows without `.zip`.
  (`f3_1_never_checked_addons_say_not_checked`, `f3_1_get_many_reports_latest_file_per_channel`,
  `f3_1_check_updates_batches_per_provider_and_isolates_failures`, `f3_1_check_updates_without_key_skips_curseforge_quietly`,
  e2e `f3_1_check_shows_update_available`)
- **F3.2 Update / Update all** ✅ — Downloads the release zip (progress in the row), validates it (no `..`/absolute paths/symlinks,
  at least one top-level folder with a `.toc`, so a wrapper-folder zip can't replace the addon), extracts next to AddOns,
  swaps the folders with backup + rollback, removes folders the new version dropped (only folders no other record lists),
  records the new version. A running update can't be started twice, also across a rescan. (e2e `f3_2_update_replaces_folder_contents`, `f3_2_update_all_updates_every_outdated_addon`, core: `f3_2_download_reports_progress_and_http_errors`, `f3_2_update_swaps_folders_and_rolls_back_on_bad_zip`, `f3_2_swap_failure_restores_backups`)
- **F3.3 Channels** 🚧 — Per addon Stable / Beta / Alpha in the detail panel (stored, rechecked); "Beta" also accepts
  stable files, etc. (`f3_1_check_updates_batches_per_provider_and_isolates_failures`)
- **F3.4 Ignore** ✅ — Ignored addons are never checked or updated; can be un-ignored; stored. Git checkouts are ignored
  automatically. (e2e `f3_4_ignored_addon_is_not_offered_for_update`)

## F4 Get Addons (remote)
- **F4.1 Search** ✅ — One field takes search text or an install link: a CurseForge addon URL, a GitHub repo URL or bare
  `owner/repo` (`providers::link_source`) shows "Install from <source>" with the resolved addon as the only result
  (Enter installs it); anything else searches CurseForge for Forever addons (debounced, stale answers dropped). Results
  show name, author, downloads, summary. Provider: `gameVersionTypeId=88568`, files filtered to Forever (no alternatives),
  mods without a Forever file dropped; categories and `dateModified` kept for the detail (`f4_1_search_curseforge_filters_forever_files`,
  `f4_4_link_source_tells_links_from_search_text`, e2e `f4_1_search_and_f4_3_install_from_a_result`,
  `f4_1_one_field_routes_links_to_install_and_text_to_search`).
- **F4.2 Popular** ✅ — Without a query, shows popular/recently updated Forever addons. Provider done:
  `POST /v1/mods/featured`, popular + recentlyUpdated deduped (`f4_2_popular_lists_featured`).
- **F4.3 Install** ✅ — Installs a search result with the default channel; it appears in My Addons and takes the folders it
  installed from other records (records left without folders go). (`f4_3_install_creates_record_and_replaces_existing_folders`,
  `f4_3_install_takes_only_shared_folders_from_other_records`, e2e `f4_1_search_and_f4_3_install_from_a_result`)
- **F4.4 Install from URL** ✅ — The F4.1 field resolves a pasted CurseForge addon URL or GitHub repo URL (releases with
  `release.json` or a `*-forever.zip`/plain zip asset) on a worker and installs it from its card or detail. Provider `from_url` done for CurseForge (slug search),
  GitHub (`owner/repo`, newest 5 non-draft releases, prerelease = Beta; plain-zip fallback skips zips tagged for other
  flavors) and WowInterface (`info<ID>-...`) (`f4_4_curseforge_url_resolves_slug`, `f4_4_github_url_picks_forever_asset`,
  `f4_4_github_plain_zip_fallback_skips_other_flavors`, e2e `f4_4_install_from_github_url_without_a_key`).
- **F4.5 Detail** ✅ — Clicking a result opens a side panel (like My Addons' detail): thumbnail, name, author, summary,
  downloads, last updated, latest Forever file (version, date, channel, game version), categories, source, website link and
  Install / progress / Installed ("Show in My Addons") / Update. The long description is fetched on a worker when it opens
  and shown as plain text, capped at 8000 characters: CurseForge's HTML (`GET /v1/mods/{id}/description`, `html::to_text`:
  tags, scripts and images dropped, paragraphs and list items kept) or a GitHub repo's README (`GET /repos/{repo}/readme`
  raw, `html::markdown_to_text`: badges, images and HTML dropped, link text kept). Esc, the close button or clicking the
  result again closes it. (`f4_5_description_is_plain_text`, `f4_5_github_readme_is_plain_text`,
  e2e `f4_5_clicking_a_result_opens_its_detail_and_esc_closes_it`, `f4_4_install_from_github_url_without_a_key`)
- **F4.6 Filters** ✅ — A category dropdown (CurseForge's WoW addon tree from `GET /v1/categories?gameId=1`, fetched once
  per run and cached 7 days in the cache dir; subcategories indented, the Class subtree left out), a row of class chips
  (only Forever's nine classes, picked by CurseForge category id, each with our own line glyph; one at a time, a second
  click clears it, a class and a dropdown category replace each other since `categoryId` takes one) and a
  sort dropdown (Popularity, Last updated, Name, Total downloads) apply to search and browsing: `categoryId`, `sortField`
  (2/3/4/6) and `sortOrder` on `/v1/mods/search`; with no text, category or order the popular list shows. Esc or a click
  outside closes a dropdown. (`f4_6_categories_are_the_addon_tree`, `f4_6_search_sends_category_and_sort`,
  e2e `f4_6_category_and_sort_filter_the_search`)
- **F4.7 WowUp Hub provider** 🚧 — `providers::Hub` (hub.wowup.io, no key, gameType `forever`, base URL in
  `ProviderConfig::hub_base_url`): search, featured (popular + recent, deduped), batch `get_many` for update checks,
  `wowup.io/addons/<id>` URLs, and folder matching by WowUp's md5 fingerprint (`POST /addons/fingerprint`, exact matches
  of a Forever release only; scans now compute it next to the CurseForge one). Only releases with a `forever` game version
  and an http(s) download count; file id = GitHub asset id, version = tag, folders from `addonFolders`. Core only: not yet
  in `providers::all` (scan/update list) or the UI. (`f4_7_hub_search_keeps_forever_releases`,
  `f4_7_hub_featured_dedupes_popular_and_recent`, `f4_7_hub_install_and_update`, `f4_7_hub_fingerprint_matches_folders`,
  `f4_7_hub_url_resolves_id`, `f2_3_fingerprint_matches_golden_value`)
- **F4.8 Multi-source search** 🚧 — `discover::search` / `discover::featured` ask every available provider in parallel
  and group the results into one entry per addon (`sources[0]` preferred: CurseForge if present, else the first provider).
  Results merge only on an exact shared id (`curse:`, `hub:`, `wowi:`, `github:<owner/repo>` lowercased; a CurseForge
  `links.sourceUrl` or Hub `repository` counts only when it is exactly a GitHub repo URL). Never on name, slug, author
  or folder; a merge that would join two different ids of one kind does not happen ("better a duplicate than a wrong
  merge"). A failing provider is reported per source, the rest still answer. Core only, no UI yet.
  (`f4_8_merges_curseforge_and_hub_on_exact_github_repo`, `f4_8_same_name_with_different_repos_is_not_merged`,
  `f4_8_conflicting_ids_are_not_merged`, `f4_8_failing_provider_returns_partial_results`)

## F5 Manage
- **F5.1 Remove** ✅ — Deletes the addon's stored folders after a confirm in the detail panel, never a folder another
  record lists. (`f5_1_remove_deletes_folders`, e2e `f5_1_remove_asks_then_deletes_the_folders`,
  `f5_1_remove_keeps_folders_another_addon_lists`)
- **F5.2 Reinstall** ✅ — Re-downloads the latest release for the channel over the addon. (`f5_2_reinstall_restores_modified_files`)
- **F5.3 Open links** ✅ — Website (`open_url`) and the addon's folder (`xdg-open`). (`f5_3_open_website`)

## F6 Maintenance
- **F6.1 Clear cache** ✅ — Deletes the client's `Cache` folder (or just `Cache/WDB`). All F6 tools refuse while a `Wow.exe` of this
  client runs (its working directory or an open file is inside the client folder; a `_forever_` command line counts only
  when `/proc` can't tell, so a game of another client never locks this one), act only on a `_forever_` client folder and never follow a `Cache` link; the UI asks first.
  (`f6_1_clear_cache_removes_cache_dir`, `f6_1_refuses_while_game_running`, `f6_1_only_touches_a_forever_client_folder`,
  e2e `f6_1_clear_cache_asks_then_deletes_the_cache`, `f6_1_maintenance_locked_while_game_runs`)
- **F6.2 Clean backups** ✅ — Deletes `WTF/**/*.lua.bak` and `WTF/*.old` leftovers. (`f6_2_clean_backup_files`)
- **F6.3 WTF backup** ✅ — Zips the `WTF` folder (settings + SavedVariables) into `<data>/wtf-backups/<installation id>`; lists backups.
  (`f6_3_backup_wtf_zips_and_lists`, e2e `f6_3_wtf_backup_is_zipped_into_the_app_data`)

## F7 Settings
- **F7.1 CurseForge API key** ✅ — Stored in Rune prefs; providers are rebuilt from it per call and saving a key rescans.
  Without a key: My Addons still scans, imports, removes, maintains and updates GitHub addons (and
  CurseForge ones from the imported last known release), shows an "Add a CurseForge key" hint; Get Addons hides the filters,
  takes GitHub links but searches nothing, with a Settings hint; nothing calls CurseForge. (`f7_1_curseforge_unavailable_without_key`,
  e2e `f7_1_without_a_key_search_is_off_and_points_to_settings`, `f7_1_f7_2_saved_settings_survive_a_restart`)
- **F7.2 Defaults** ✅ — Default channel for new addons, stored in prefs. (e2e `f7_1_f7_2_saved_settings_survive_a_restart`)
- **F7.3 Themes** ✅ — Settings → Appearance picks the look (Moonlit; Hearthwood: carved wood, stone and gold; Emberglow: mossy
  black stone, glowing gold, framed cards and small-caps serif controls; Goblin Ledger: calm charcoal menus, ochre gold;
  Grimoire: the classic in-game book windows, parchment pages with dark ink in a dark stone frame with gold rims, a
  red-brown title banner, item-slot tiles, red lacquered buttons, its own tome/scroll/anvil/cog/chest glyphs and one light
  from the top left: raised parts lit and casting down-right, fields sunk, pressed buttons darken). Cards (five on one
  row) preview each theme in its own skin, the choice applies at once, is kept in prefs (`theme`) and read before the
  first frame; an unknown stored id falls back to Moonlit. Layout is the same in every theme; every theme keeps text at
  4.5:1 and dimmed or status ink at 3:1 on its window and cards. (e2e `f7_3_theme_choice_applies_and_survives_restart`,
  `f7_3_unknown_stored_theme_falls_back_to_moonlit`, `f7_3_every_theme_renders_every_screen`; unit
  `f7_3_every_theme_keeps_text_readable`)

## App wiring ✅
`crates/app/src/core_backend.rs` implements `backend::Backend` over the core: data/cache dirs from `rune::dirs::AppDirs`
(`RUNE_DIRS_ROOT` relocates them), settings in `rune::prefs::Prefs`, provider endpoints overridable with
`ADDONS_CURSEFORGE_URL` / `ADDONS_GITHUB_URL` / `ADDONS_WOWINTERFACE_URL`. The window uses it; headless binary runs
(`--snap`, `--script`) use the deterministic `MockBackend` (scenes) unless `ADDONS_BACKEND=core`.
End-to-end specs in `crates/app/tests/e2e_*.rs` drive the real UI and core against a temp Forever tree, a fake import
config and a FakeServer (helpers in `tests/common/mod.rs`); `tests/ui_shell.rs` keeps the mock-only UI checks
(`navigation_switches_screens`, `f2_1_filter_narrows_the_list`, `slash_focuses_the_filter`, `f3_2_failed_update_shows_error`,
`f5_3_open_website`, `f6_1_maintenance_locked_while_game_runs`,
`late_results_never_land_on_the_wrong_installation_or_restart_updates`: worker results carry their installation id and
are dropped after a switch).

## UI shell ✅
Rune UI in `crates/app` (themes in `src/theme/`, default Moonlit): sidebar nav with update badge and
installation picker, header band (stars in Moonlit), My Addons (filter, virtual list, detail panel), Get Addons
(one search-or-link field, category and sort dropdowns, popular list, result detail panel, no-key hint), Maintenance (confirm + result per
card, locked while the game runs), Settings (theme, API key, default channel, installations, WowUp import),
toasts and onboarding. Every scene has a `<scene>@<theme>` variant (`--snap x.png --scene settings@hearthwood`).

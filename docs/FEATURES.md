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
  each with its own addon list; the sidebar picker switches, and the choice is kept in prefs. An installation is named
  "WoW Forever"; only while two share that name does each show what tells them apart (its Wine prefix, "WoW Forever
  (battlenet)", else its place in the list). (`f1_3_installations_persist`, `f1_3_installation_names_show_the_prefix_only_for_twins`)

## F2 My Addons (local)
- **F2.1 Scan** ✅ — Reads `Interface/AddOns`, picks the right TOC per folder (Forever: `_Forever`/`_Camelot`
  first, then the plain TOC), and reads title, version, author, interface version. Rows show provider thumbnails
  (downloaded on a worker, cached in the cache dir; CurseForge's only in memory, its terms forbid storing its data) over an initials tile. (`f2_1_scan_picks_forever_toc`,
  e2e `f2_1_f2_3_scan_shows_matched_and_unknown_addons`, `f2_1_thumbnails_load_over_initials`)
- **F2.2 Grouping** ✅ — Folders that belong to one addon (same provider match, folders listed in the installed file,
  or unmatched folders requiring a prefix parent such as `Details_*`) form one entry. (`f2_2_groups_by_provider_and_dependencies`)
- **F2.3 Matching** ✅ — Folders are matched to CurseForge via fingerprints (murmur2), to WowUp Hub via
  its md5 fingerprints (F4.7, no key needed)
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
- **F2.6 Sort** ✅ — The Addon, Game and Status column headers are buttons (keyboard: Space) that order the list by
  that column and flip the order when pressed again; a chevron marks the active one. Addon: name A–Z; Game: oldest game
  version first (dotted numbers compared part by part, none last); Status: what needs the user first (error, update,
  working, not checked, up to date, unknown, ignored), ties by name. Version is not sortable (authors' own strings;
  Status covers "outdated"). Applies after the filter text; the order is kept in prefs (`addon_order`, e.g. `status-asc`).
  (`f2_6_column_headers_sort_the_list`)

## F3 Updates
- **F3.1 Check for updates** ✅ — After each scan and on demand, `updates::check_updates` asks each available provider
  once for all managed (linked, not ignored) addons and marks "Update available"; a failing provider (or GitHub repo) is
  reported without stopping the rest, providers without a key are skipped. A managed addon no check has answered for
  (no `latest`, e.g. CurseForge without a key) shows a muted "Not checked", never "Up to date", and without a key the header says
  "CurseForge updates not checked" (the imported last known releases are all it has). A CurseForge display name that is a zip name shows without `.zip`.
  (`f3_1_never_checked_addons_say_not_checked`, `f3_1_get_many_reports_latest_file_per_channel`,
  `f3_1_check_updates_batches_per_provider_and_isolates_failures`, `f3_1_check_updates_without_key_skips_curseforge_quietly`,
  e2e `f3_1_check_shows_update_available`)
- **F3.2 Update / Update all** ✅ — Downloads the release zip (progress in the row) straight into a temp file in the
  cache, so big addons (audio packs pass 512 MB) never sit in memory, validates it (no `..`/absolute paths/symlinks,
  at least one top-level folder with a `.toc`, so a wrapper-folder zip can't replace the addon), extracts next to AddOns,
  swaps the folders with backup + rollback, removes folders the new version dropped (only folders no other record lists),
  records the new version. A running update can't be started twice, also across a rescan. CurseForge CDN downloads carry the
  user's key (`x-api-key`), sent only to CurseForge hosts and checked again on every redirect. (e2e `f3_2_update_replaces_folder_contents`, `f3_2_update_all_updates_every_outdated_addon`, core: `f3_2_download_reports_progress_and_http_errors`, `f3_2_update_swaps_folders_and_rolls_back_on_bad_zip`, `f3_2_swap_failure_restores_backups`, `f3_2_big_downloads_stream_to_a_file`, `f3_2_download_keys_go_only_to_their_hosts_even_across_redirects`, `f3_2_only_curseforge_hosts_get_the_key`)
- **F3.3 Channels** 🚧 — Per addon Stable / Beta / Alpha in the detail panel (stored, rechecked); "Beta" also accepts
  stable files, etc. (`f3_1_check_updates_batches_per_provider_and_isolates_failures`)
- **F3.4 Ignore** ✅ — Ignored addons are never checked or updated; can be un-ignored; stored. Git checkouts are ignored
  automatically. (e2e `f3_4_ignored_addon_is_not_offered_for_update`)
- **F3.5 Updates only the source hands out** ✅ — When the newest file has no download link (the CurseForge author
  allows downloads only through CurseForge's own app or website), the row's Update button carries an external-link
  glyph (the reason in its tooltip) and, like the detail's "Get it on CurseForge", opens that file's download page
  (`<addon page>/download/<file id>`), where the browser starts the download; other sources open the addon's page.
  "Update all" leaves these out. The row then says "Waiting for download…" (Cancel stops it; it gives up after ten
  minutes) while a worker looks in the Downloads folder (`rune::dirs`) once a second for a `.zip` with exactly the
  size and SHA-1 CurseForge publishes (`manual::find_in`; unfinished `.part`/`.crdownload` files never match). The
  first look runs before the browser opens: a copy already there is installed without a tab. Found, it is installed
  like any update (validation, backup, rollback) and deleted with every identical copy (`Foo(1).zip`). Saved elsewhere: "Install from file…"
  in the detail picks the zip, checks it the same way, installs it and leaves it in place. The size and hash stay in
  memory only (#69). (core `f3_5_finds_only_the_exact_zip_in_downloads`,
  `f3_5_installs_a_zip_fetched_by_hand_only_when_it_is_the_file`, `f3_5_curseforge_file_carries_its_size_and_sha1`,
  e2e `f3_5_update_only_on_curseforge_installs_the_browser_download`,
  `f3_5_a_zip_already_in_downloads_installs_without_the_browser`, `f3_5_waiting_can_be_cancelled_and_ends_by_itself`,
  `f3_5_install_from_file_checks_the_zip_first`)

## F4 Get Addons (remote)
- **F4.1 Search** ✅ — One field takes search text or an install link: a CurseForge addon URL, a WowUp Hub page
  (`wowup.io/addons/<id>`), a GitHub repo URL or bare `owner/repo` (`providers::link_source`) shows "Install from
  <source>" with the resolved addon as the only result (Enter installs it); anything else searches every available
  source for Forever addons (F4.8; debounced, stale answers dropped). Results show name, author, downloads, summary and
  their sources. Provider: `gameVersionTypeId=88568`, files filtered to Forever (no alternatives),
  mods without a Forever file dropped; categories and `dateModified` kept for the detail (`f4_1_search_curseforge_filters_forever_files`,
  `f4_4_link_source_tells_links_from_search_text`, e2e `f4_1_search_and_f4_3_install_from_a_result`,
  `f4_1_one_field_routes_links_to_install_and_text_to_search`).
- **F4.2 Popular** ✅ — Without a query, shows popular/recently updated Forever addons from every available source,
  grouped like search (`discover::featured`). CurseForge: `POST /v1/mods/featured`, popular + recentlyUpdated deduped
  (`f4_2_popular_lists_featured`, e2e `f4_8_a_result_on_two_sources_installs_from_the_picked_one`).
- **F4.3 Install** ✅ — Installs a search result with the default channel; it appears in My Addons and takes the folders it
  installed from other records (records left without folders go). When the author allows downloads only in CurseForge's own app
  or website (no `downloadUrl`), Culm never builds a URL itself: Install carries an external-link glyph and fetches the file
  by hand like an F3.5 update (looks in Downloads first, else opens the file's download page, "Waiting for download…" with
  Cancel on the card and in the detail, installs and deletes the zip once it lands; the detail adds "Install from file…").
  Without the file's size and SHA-1 Install is greyed out with the reason in its tooltip.
  (`f4_3_install_creates_record_and_replaces_existing_folders`, `f4_3_install_takes_only_shared_folders_from_other_records`,
  `f4_3_no_download_url_means_the_author_said_no`, `f4_3_installs_a_new_addon_from_a_zip_fetched_by_hand`, e2e
  `f4_1_search_and_f4_3_install_from_a_result`, `f4_3_install_only_on_curseforge_installs_the_browser_download`,
  `f4_3_a_zip_already_in_downloads_installs_without_the_browser`, `f4_3_waiting_for_a_new_addon_can_be_cancelled`,
  `f4_3_install_from_file_checks_the_zip_first`, `f4_3_install_is_greyed_out_when_culm_cannot_recognise_the_file`)
- **F4.4 Install from URL** ✅ — The F4.1 field resolves a pasted CurseForge addon URL, WowUp Hub page or GitHub repo URL (releases with
  `release.json` or a `*-forever.zip`/plain zip asset) on a worker and installs it from its card or detail. Provider `from_url` done for CurseForge (slug search),
  GitHub (`owner/repo`, newest 5 non-draft releases, prerelease = Beta; plain-zip fallback skips zips tagged for other
  flavors) and WowInterface (`info<ID>-...`) (`f4_4_curseforge_url_resolves_slug`, `f4_4_github_url_picks_forever_asset`,
  `f4_4_github_plain_zip_fallback_skips_other_flavors`, e2e `f4_4_install_from_github_url_without_a_key`).
- **F4.5 Detail** ✅ — Clicking a result opens a side panel (like My Addons' detail): thumbnail, name, author, summary,
  downloads, last updated, latest Forever file (version, date, channel, game version), categories, source, website link and
  Install / progress / Installed ("Show in My Addons") / Update. The long description is fetched on a worker when it opens
  and shown as plain text, capped at 8000 characters: CurseForge's HTML (`GET /v1/mods/{id}/description`, `html::to_text`:
  tags, scripts and images dropped, paragraphs and list items kept) or, for GitHub and WowUp Hub addons, the repo's README
  (`GET /repos/{repo}/readme` raw, `html::markdown_to_text`: badges, images and HTML dropped, link text kept). With several
  sources the panel describes the picked one (F4.8). Esc, the close button or clicking the
  result again closes it. (`f4_5_description_is_plain_text`, `f4_5_github_readme_is_plain_text`,
  e2e `f4_5_clicking_a_result_opens_its_detail_and_esc_closes_it`, `f4_4_install_from_github_url_without_a_key`)
- **F4.6 Filters** ✅ — A category dropdown (CurseForge's WoW addon tree from `GET /v1/categories?gameId=1`, fetched once
  per run and never stored, per CurseForge's terms; subcategories indented, the Class subtree left out), a row of class chips
  (only Forever's nine classes, picked by CurseForge category id, each with its round medallion: a bevelled metal ring
in the theme's tones, gilded on the active chip, around a disc showing the original WoW class icon cut round (only its baked 2 px frame is cropped; the ring never covers the picture); one at a time, a second
  click clears it, a class and a dropdown category replace each other since `categoryId` takes one) and a
  sort dropdown (Popularity, Last updated, Name, Total downloads) apply to search and browsing: `categoryId`, `sortField`
  (2/3/4/6) and `sortOrder` on `/v1/mods/search`; with no text, category or order the popular list shows. Esc or a click
  outside closes a dropdown. The section heading names a non-default order ("WARRIOR | LAST UPDATED", set apart by a hairline). While a class,
  category or order other than Popularity is set, "Clear filters" beside the dropdowns resets all three at once; the
  search text stays (it is the query, not a filter on it) and Esc only closes menus and the detail panel. Filters are
  CurseForge's: while a category or class is set (or the list is browsed in another order without text) only CurseForge
  is asked, silently. (`f4_6_categories_are_the_addon_tree`, `f4_6_search_sends_category_and_sort`,
  e2e `f4_6_category_and_sort_filter_the_search`, `f4_6_clear_filters_resets_class_category_and_sort`)
  - **F4.6.1 Class icons** ✅ — The icons are Blizzard's art, so they are never bundled: `class_icons::load` downloads
    them on a worker the first time the class chips show (never at startup otherwise), from the first source that serves
    a valid image (decodes, < 200 KB, 32–128 px): Blizzard's render CDN (`classic-us`, then `us`);
    `ADDONS_CLASS_ICON_URLS` (comma-separated base URLs) replaces the list. They are kept for good in
    `<data>/class-icons/<slug>.jpg` (not the cache dir, which cleaners empty), unaltered; later starts read them from disk without a request, a missing or corrupt file is
    fetched again. A source that fails is skipped for the rest of the run and nothing is retried until the next start;
    without an icon the medallion keeps our own class glyph on the darkened class colour, which also shows while the
    icon decodes. (`f4_6_class_icons_come_from_the_first_working_source`, `f4_6_cached_class_icons_need_no_request`,
    `f4_6_corrupt_cached_class_icon_is_fetched_again`, `f4_6_class_icons_unavailable_cost_one_attempt_per_source`,
    `f4_6_oversized_class_icon_is_refused`, e2e `f4_6_class_chips_show_the_original_icons`,
    `f4_6_class_chips_keep_their_glyphs_without_icons`)
- **F4.7 WowUp Hub provider** ✅ — `providers::Hub` (hub.wowup.io, no key, gameType `forever`, base URL in
  `ProviderConfig::hub_base_url`): search, featured (popular + recent, deduped), batch `get_many` for update checks,
  `wowup.io/addons/<id>` URLs, and folder matching by WowUp's md5 fingerprint (`POST /addons/fingerprint`, exact matches
  of a Forever release only; scans now compute it next to the CurseForge one). Only releases with a `forever` game version
  and an http(s) download count; file id = GitHub asset id, version = tag, folders from `addonFolders`. In
  `providers::all` between CurseForge and GitHub (scans, update checks, search); `ADDONS_HUB_URL` points it elsewhere. (`f4_7_hub_search_keeps_forever_releases`,
  `f4_7_hub_featured_dedupes_popular_and_recent`, `f4_7_hub_install_and_update`, `f4_7_hub_fingerprint_matches_folders`,
  `f4_7_hub_url_resolves_id`, `f2_3_fingerprint_matches_golden_value`)
- **F4.8 Multi-source search** ✅ — `discover::search` / `discover::featured` ask every available provider in parallel
  and group the results into one entry per addon (`sources[0]` preferred: CurseForge if present, else the first provider).
  Results merge only on an exact shared id (`curse:`, `hub:`, `wowi:`, `github:<owner/repo>` lowercased; a CurseForge
  `links.sourceUrl` or Hub `repository` counts only when it is exactly a GitHub repo URL). Never on name, slug, author
  or folder; a merge that would join two different ids of one kind does not happen ("better a duplicate than a wrong
  merge"). A failing provider is reported per source, the rest still answer. Get Addons shows one card per entry (the
  preferred source's facts, "CurseForge | also on WowUp Hub"); its Install uses the preferred source. The detail panel
  lists every source (latest version, downloads) as a picker, preferred first, and installs from the picked one. A
  source that fails is a one-line note over the others' results; only when all fail is it an error. A result counts as
  installed when one of its sources is, or when an installed addon already has all of its folders. CurseForge also
  gets the sort order; other sources' hits are merged and the list re-sorted by the order picked.
  (`f4_8_merges_curseforge_and_hub_on_exact_github_repo`, `f4_8_same_name_with_different_repos_is_not_merged`,
  `f4_8_conflicting_ids_are_not_merged`, `f4_8_failing_provider_returns_partial_results`,
  e2e `f4_8_a_result_on_two_sources_installs_from_the_picked_one`,
  `f4_8_a_source_that_fails_is_a_note_and_the_rest_still_works`, `f4_1_search_and_f4_3_install_from_a_result`)

## F5 Manage
- **F5.1 Remove** ✅ — Deletes the addon's stored folders after a confirm in the detail panel, never a folder another
  record lists. (`f5_1_remove_deletes_folders`, e2e `f5_1_remove_asks_then_deletes_the_folders`,
  `f5_1_remove_keeps_folders_another_addon_lists`)
- **F5.2 Reinstall** ✅ — Re-downloads the latest release for the channel over the addon. (`f5_2_reinstall_restores_modified_files`)
- **F5.3 Open links** ✅ — Website (`open_url`) and the addon's folder in the file manager (`explorer` on Windows, `open` on macOS,
  `xdg-open` on Linux). (`f5_3_open_website`)

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
  shows and searches WowUp Hub only, takes links, and a hint says a key adds CurseForge; nothing calls CurseForge.
  (`f7_1_curseforge_unavailable_without_key`, e2e `f7_1_without_a_key_the_hub_answers_and_settings_add_curseforge`,
  `f7_1_f7_2_saved_settings_survive_a_restart`)
- **F7.2 Defaults** ✅ — Default channel for new addons, stored in prefs. (e2e `f7_1_f7_2_saved_settings_survive_a_restart`)
- **F7.3 Themes** ✅ — Settings → Appearance picks the look (Moonlit; Hearthwood: carved wood, stone and gold; Emberglow: mossy
  black stone, glowing gold, framed cards and small-caps serif controls; Goblin Ledger: calm charcoal menus, ochre gold;
  Grimoire: the classic in-game book windows, parchment pages with dark ink in a dark stone frame with gold rims, a
  red-brown title banner, item-slot tiles, red lacquered buttons, its own tome/scroll/anvil/cog/chest glyphs and one light
  from the top left: raised parts lit and casting down-right, fields sunk, pressed buttons darken). Cards (five on one
  row) preview each theme in its own skin (a miniature window: sidebar with the brand mark, slim band, buttons), the choice applies at once, is kept in prefs (`theme`) and read before the
  first frame; an unknown stored id falls back to Moonlit. Layout is the same in every theme; every theme keeps text at
  4.5:1 and dimmed or status ink at 3:1 on its window and cards. Every font is bundled (`theme::FONTS`, OFL, in
  `crates/app/assets/fonts`): Inter, JetBrains Mono for versions and paths, Cinzel (Hearthwood, Emberglow, Grimoire
  titles), Marcellus (Ledger titles) and EB Garamond (Grimoire notes), so each theme looks the same on every OS; each
  family a theme names resolves to a bundled face, never a system one. (e2e `f7_3_theme_choice_applies_and_survives_restart`,
  `f7_3_unknown_stored_theme_falls_back_to_moonlit`, `f7_3_every_theme_renders_every_screen`,
  `f7_3_every_theme_font_is_bundled`; unit
  `f7_3_every_theme_keeps_text_readable`)
- **F7.4 Name and folders** ✅ — The app is Culm (`brand.rs`: name, tagline, app id `dev.culm.app`).
- **F7.5 About** ✅ — Settings → About shows the version from `Cargo.toml` and, until 1.0, the release stage
  (`brand::STAGE`, "v0.1.0 · Beta"). (`f7_5_about_shows_the_version_and_release_stage`)

## F8 Releases
- **F8.1 Versioning** ✅ — SemVer, 0.x while in beta; user-visible changes go into `CHANGELOG.md` under "Unreleased".
- **F8.2 CI** ✅ — `.github/workflows/ci.yml` runs `cargo fmt --check`, `cargo clippy --all-targets -D warnings` and
  `cargo test` on Linux, Windows and macOS for every push and pull request (fmt once, on Linux), then saves headless
  `--snap` screenshots of a few scenes per OS as the `screenshots-<OS>` workflow artifact.
- **F8.3 Linux AppImage** ✅ — Pushing a tag `v<version>` (it must match `Cargo.toml`) runs `.github/workflows/release.yml`:
  tests, then a release build on Ubuntu 22.04 (glibc 2.35, so older distros run it), `packaging/appimage.sh` packs
  `Culm-<version>-x86_64.AppImage` with the desktop entry, icons and AppStream metadata, and embeds the update
  information `gh-releases-zsync|gengurke|Culm|latest|Culm-*-x86_64.AppImage.zsync`. The release carries every
  platform's files (F8.5), `SHA256SUMS` over all of them, and notes from the CHANGELOG section plus the install steps
  in `packaging/release-install.md`. Releases are never marked "pre-release": zsync's `latest` skips those, and the
  beta shows in the version instead. Run by hand (`workflow_dispatch`), the workflow builds every package from a
  branch as workflow artifacts and publishes nothing.
- **F8.4 Updates itself** ✅ — Run as an AppImage (`$APPIMAGE`), Culm asks GitHub for its newest release once per start
  (`self_update::latest`, no other data sent); a newer one shows a toast and an "Update to x.y.z" button in the sidebar and
  under Settings → About. Update downloads the asset next to the AppImage, checks it against the release's
  `SHA256SUMS` (no checksum, no install), makes it executable and moves it over the AppImage in one step; "Restart for
  x.y.z" starts the new file and quits; the new process gets no inherited descriptors, so the old image's mount (the
  AppImage runtime keeps it through fd 1023) ends with the old process. A failed or damaged download leaves the running version alone and can be retried.
  The Windows and macOS release builds also check at every start but can't replace themselves: their "Update to x.y.z"
  opens the release page. Other builds (from source) check only when "Check for updates" is pressed and link the
  release page the same way.
  `ADDONS_APP_RELEASES_URL` points the check elsewhere. (`f8_4_latest_release_picks_the_fitting_asset`,
  `f8_4_update_replaces_the_file_and_keeps_it_executable`, `f8_4_damaged_download_leaves_the_app_alone`,
  `f8_4_release_without_checksums_is_refused`, e2e `f8_4_update_from_the_sidebar_replaces_the_appimage_and_restarts_it`,
  `f8_4_damaged_update_keeps_the_running_version`, `f8_4_a_copy_that_cant_update_itself_asks_only_when_told_and_links_the_release`,
  `f8_4_a_windows_or_macos_build_checks_at_start_and_links_the_release`, `f8_4_check_says_when_the_app_is_up_to_date`)
- **F8.5 Windows and macOS packages** ✅ — The same tag builds, besides the AppImage:
  - **Windows** (x64, MSVC, C runtime linked in): `culm.exe` carries the icon and version info (`crates/app/build.rs`,
    `assets/icon/culm.ico`; `packaging/icons.sh` makes it and `culm.icns` from `packaging/culm.svg`) and opens no console window. `packaging/windows/culm.iss` (Inno Setup) makes
    `Culm-<version>-windows-x64-setup.exe`: per-user install without admin rights into `%LOCALAPPDATA%\Programs\Culm`,
    Start menu entry, optional desktop icon, uninstaller (leaves settings and addon records), installs over an older
    version.
  - **macOS** (13+): `packaging/macos.sh` joins the arm64 and x86_64 builds with `lipo` into a universal `Culm.app`
    (bundle id `dev.culm.app` from `brand.rs`, the committed `culm.icns`), signs it ad hoc and packs it into
    `Culm-<version>-macos.dmg` with an Applications link.
  - Neither is signed with a paid certificate yet: Windows SmartScreen needs "More info → Run anyway" once, macOS
    "Open Anyway" in Privacy & Security once per downloaded version (README, release notes). Checked by the workflow run, not by tests.

## App wiring ✅
`crates/app/src/core_backend.rs` implements `backend::Backend` over the core: data/cache dirs from `rune::dirs::AppDirs`
(`RUNE_DIRS_ROOT` relocates them), settings in `rune::prefs::Prefs`, provider endpoints overridable with
`ADDONS_CURSEFORGE_URL` / `ADDONS_GITHUB_URL` / `ADDONS_WOWINTERFACE_URL`. The window uses it; headless binary runs
(`--snap`, `--script`) use the deterministic `MockBackend` (scenes) unless `ADDONS_BACKEND=core`.
End-to-end specs in `crates/app/tests/e2e_*.rs` drive the real UI and core against a temp Forever tree, a fake import
config and a FakeServer (helpers in `tests/common/mod.rs`); `tests/ui_shell.rs` keeps the mock-only UI checks
(`navigation_switches_screens`, `f2_1_filter_narrows_the_list`, `f2_6_column_headers_sort_the_list`, `slash_focuses_the_filter`, `f3_2_failed_update_shows_error`,
`f5_3_open_website`, `f6_1_maintenance_locked_while_game_runs`,
`late_results_never_land_on_the_wrong_installation_or_restart_updates`: worker results carry their installation id and
are dropped after a switch).

## UI shell ✅
Rune UI in `crates/app` (themes in `src/theme/`, default Moonlit): sidebar nav with update badge and
installation picker, a slim header band per page (title, muted meta pieces set apart by hairlines, actions at the right;
stars in Moonlit; My Addons: addon count and update state, the game version stays in the picker), My Addons (filter, sortable column headers, virtual list, detail panel), Get Addons
(one search-or-link field, category and sort dropdowns, class medallion chips, clear filters, popular list, result detail panel, no-key hint), Maintenance (confirm + result per
card, locked while the game runs), Settings (theme, API key, default channel, installations, WowUp import),
toasts and onboarding. Meaningful actions explain themselves in a short tooltip after a rest of the pointer, drawn in
the theme's floating (menu) look (`ui_buttons_explain_themselves`). Active nav rows are inset plaques and the selected row a
tinted line in a thin ring: no coloured side stripes. Toasts are framed alerts; five other looks are kept as
alternatives (`ADDONS_TOAST_STYLE=0..5`, scene `toasts`). The sidebar wordmark is "Culm" over "Addon Manager" in Inter. Every scroll area (`ui::scroll`) has a thin overlay scroll bar: hidden at rest, shown while the pointer is over it or it
scrolls, a 3 pt pill in the theme's `border_strong` that widens to 6 pt in its `accent` under the pointer (`ui_every_page_scrolls_with_a_thin_scroll_bar`).
Every scene has a `<scene>@<theme>` variant (`--snap x.png --scene settings@hearthwood`).

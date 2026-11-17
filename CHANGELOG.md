# Changelog

User-visible changes per release. Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
versions follow [SemVer](https://semver.org/). Until 1.0 every release is a beta: it works, but may still change
or break. New changes go under "Unreleased"; a release turns that heading into its version and date.

## [Unreleased]

## [0.1.0] - 2026-11-17

First beta, Linux only (AppImage).

### Added
- Finds World of Warcraft: Forever clients in Wine, Proton, Faugus, Lutris and Bottles prefixes; more can be added by hand.
- My Addons: scans `Interface/AddOns`, groups folders into addons, matches them to CurseForge, WowUp Hub and GitHub,
  and updates or removes them.
- Get Addons: one search over CurseForge (with your own API key) and WowUp Hub; install from a GitHub link.
- Imports what WowUp already installed.
- Maintenance: clears the game cache and leftover backup files, and backs up the settings folder.
- Five themes: Moonlit, Hearthwood, Emberglow, Ledger and Grimoire.
- The AppImage carries update information, so Gear Lever, AppImageUpdate and AppImageLauncher can update it.

[Unreleased]: https://github.com/gengurke/Culm/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/gengurke/Culm/releases/tag/v0.1.0

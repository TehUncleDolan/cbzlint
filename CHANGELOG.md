# Changelog

This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Possible sections are:

- `Added` for new features.
- `Changed` for changes in existing functionality.
- `Deprecated` for soon-to-be removed features.
- `Removed` for now removed features.
- `Fixed` for any bug fixes.
- `Security` in case of vulnerabilities.

<!-- next-header -->

## [Unreleased] - ReleaseDate

## [0.3.0] - 2023-01-04

### Changed

- Revert Searx (too flaky)

## [0.2.6] - 2021-05-19

### Changed

- Update the blacklist
- Only check date for the expected last modified time

### Fixed

- Fix code for Rust 1.52
- Handle non-digital release (e.g. scan)

## [0.2.5] - 2021-05-06

### Changed

- Update the blacklist

## [0.2.4] - 2021-04-29

### Changed

- Blacklist more servers

### Fixed

- Don't crash on volume overflow

## [0.2.3] - 2021-04-29

### Changed

- Blacklist https://search.mdosch.de/

### Fixed

- Restore original deps ^^"

## [0.2.2] - 2021-04-29

### Changed

- Blacklist https://search.modalogi.com/

### Fixed

- Restore correct regexp support

## [0.2.1] - 2021-04-29

### Fixed

- Restore HTTPS support

## [0.2.0] - 2021-04-29

### Changed

- Use Searx to find books on bedetheque, a lot more reliable than the previous
  approach

## [0.1.3] - 2021-04-23

### Added

- Add an empty line between each book, makes output more legible
- Check that the last modified time of the images is `2000-01-01 00:00:01`
- Check against the presence of EXIF metadata

### Changed

- Do not print one message per page whose size does not match
- Be more tolerant for DPR width (allows +/- 10% of variation)

### Fixed

- Hyphen handling in series' title

## [0.1.2] - 2021-04-22

### Fixed

- Ignore hyphen in series' title

## [0.1.1] - 2021-04-20

### Added

- Display which bedetheque page has been used to check metadata
- Improve romanization handling (fuzzy matching)
- New logic to find the right book on bedetheque, more robust

### Changed

- Authors list comparison is case-insensitive

## [0.1.0] - 2021-04-20

### Added

- Check that image resolutions inside the CBZ match the name
- Check the publication year in the name against bedetheque
- Check the author list (and the order) in the name against bedetheque

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.6] - 2026-07-24

### Fixed

- Handled cyclic and dangling symbolic links safely during recursive directory operations, preventing infinite loops and unexpected errors (#3).
- Fixed CLI target validation to allow opening broken or cyclic symbolic links.
- Used `lchown` instead of `chown` when modifying ownership to avoid dereferencing symbolic links.

### Changed

- Enhanced recursive directory processing to collect and report item errors gracefully as a summary instead of aborting on the first error.

## [1.0.5] - 2026-07-21

### Added

- Initialized `CHANGELOG.md` following the Keep a Changelog format.
- Automated release note generation from `CHANGELOG.md` version headers in the release workflow.
- Added support for allowing dirty working directories in `dist-workspace.toml` for CI workflows.

### Changed

- Expanded the test suite to improve project coverage.
- Enhanced the CI pipeline to run:

  - `cargo fmt --check`
  - `cargo clippy -- -D warnings`
  - `cargo test`
  - `cargo audit`
- Updated the release workflow to publish release notes directly from the corresponding `CHANGELOG.md` version section.

## [1.0.4] - 2026-07-21

### Added
- Standardized GitHub issue templates (`bug_report.yml`, `feature_request.yml`, `documentation.yml`, `performance.yml`).
- Pull request template (`pull_request_template.md`) to streamline contributions.
- Project status and CI badges in `README.md`.

### Changed
- Refactored permission methods to take ownership for improved performance and safety.
- Applied consistent formatting across the TUI layer and main application loop.

### Fixed
- Corrected typo in edit octal UI label.

## [1.0.3] - 2026-07-20

### Added
- Initial release of `rwx` terminal permission inspector and editor.

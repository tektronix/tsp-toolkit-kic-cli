# Change Log

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!--
Check [Keep a Changelog](http://keepachangelog.com/) for recommendations on how to structure this file.

    Added -- for new features.
    Changed -- for changes in existing functionality.
    Deprecated -- for soon-to-be removed features.
    Removed -- for now removed features.
    Fixed -- for any bug fixes.
    Security -- in case of vulnerabilities.
-->
## [0.15.3]

- Fix issue where unrecognized model number causes kic-cli to never exit (TSP-645)

## [0.15.1]

### Changed
- **tsp-toolkit-kic-lib:** Clean up instrument connections when an AsyncStream
  stream is dropped

### Fixed
- Remove errors when fetching nodes with `.nodes` command

### Security
- Bump `h2` version

## [0.15.0]

### Fixed
- Change language to `TSP` after connection to TTI instrument (TSP-561)
- **tsp-toolkit-kic-lib:** Use `*TST?` to check login state instead of
  `print("unlocked")` just in case we are in a SCPI command set mode.
- Fix script name issues if the name contains special characters (TSP-505)

## [0.14.1]

### Changed
- Prepend `kic_` to scripts loaded by kic_cli to prevent name-collisions (TSP-505)

### Fixed
- Update Dependencies (TSP-576)


## [0.13.2]

### Fixed
- Fixed crash when binary delimiter (`#0`) is encountered in instrument output (TSP-544)
- Truncate old file content in the node configuration file (TSP-533)
- Fix issue with `update` subcommand exiting too soon (TSP-572, TSP-573)

## [0.13.0]

### Fixed

- Terminal closes when sending invalid TSP (TSP-513)

## [0.12.2]

### Fixed

- Fixed Fatal Error due to firmware limitation on TTI instruments (TSP-415)
- Fixed instrument connection failed (TSP-486)

## [0.12.1]

### Changed

- Restore password hide feature back after ki-comms refactor (TSP-363)
- Implement Password prompt (TSP-480)
-
### Fixed

- Extension wants a password when there isn't one (TSP-416)

## [0.12.0]

### Added
- Add message when starting FW upgrade (TSP-455)

## [0.11.2]

### Added
- Feature to retrieve TSP-Link network details

<!--Version Comparison Links-->
[Unreleased]: https://github.com/tektronix/tsp-toolkit-kic-cli/compare/v0.15.1...HEAD
[0.15.1]: https://github.com/tektronix/tsp-toolkit-kic-cli/releases/tag/v0.15.1
[0.15.0]: https://github.com/tektronix/tsp-toolkit-kic-cli/releases/tag/v0.15.0
[0.14.1]: https://github.com/tektronix/tsp-toolkit-kic-cli/releases/tag/v0.14.1
[0.13.2]: https://github.com/tektronix/tsp-toolkit-kic-cli/releases/tag/v0.13.2
[0.13.0]: https://github.com/tektronix/tsp-toolkit-kic-cli/releases/tag/v0.13.0
[0.12.2]: https://github.com/tektronix/tsp-toolkit-kic-cli/releases/tag/v0.12.2
[0.12.1]: https://github.com/tektronix/tsp-toolkit-kic-cli/releases/tag/v0.12.1
[0.12.0]: https://github.com/tektronix/tsp-toolkit-kic-cli/releases/tag/v0.12.0
[0.11.2]: https://github.com/tektronix/tsp-toolkit-kic-cli/releases/tag/v0.11.2

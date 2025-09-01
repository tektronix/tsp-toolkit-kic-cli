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

## [0.21.0]
- Consolidate kic-debug

## [0.20.1]

### Fixed
- Getting error after firmware file transferred successfully
- Firmware upgrade for TTI and 2600 models getting stuck over slow connection speeds

## [0.20.0]

### Added
- .abort command to cancel any ongoing job
- Support for passwords for all instruments and connection types

### Changed
- CLI arguments now only require IP Address/VISA Resource String and don't need a `lan` or `visa` connection type specifier

### Fixed
- Unable to fetch TSPLink network information from Trebuchet

## [0.19.8]

### Changed
- Sanitize log files to remove sensitive information.

### Fixed
- Unable to fetch TSPLink network information from Trebuchet

## [0.19.7]

### Fixed
- Recognize when socket has been terminated from device-side more quickly and close.

### Changed

- Updated branding from Keithley Instruments to Tektronix

## [0.19.6]

### Fixed
- (**tsp-toolkit-kic-lib**) Progress indicators for 2600, 3706, and TTI instruments

## [0.19.5]

### Added

- (**tsp-toolkit-kic-lib**) Progress indicators for very large scripts and firmware files

### Changed

- JSON structure updated to include module info
- Write fetched configuration to setting.json file
- (**tsp-toolkit-kic-lib**) No longer need to call `slot.stop` and `slot.start` since that is done by firmware now

### Fixed

- (**tsp-toolkit-kic-lib**) Issues with firmware updates over USBTMC on some instruments

## [0.19.4]

- Make the `script` subcommand print output and close after completion

## [0.19.3]

### Changed

- Reduce amount of default logging
- Use new `firmware.valid` attribute to gate firmware update

### Added

- Added connection support for missing instrument models

## [0.19.2]

### Fixed

- Unable to connect over LAN, while accessing instrument remotely
- Fix upgrade procedure for trebuchet


## [0.19.1]

### Changed

- Determining supported instruments now depends on `kic-lib` implementation, not
  a local re-implementation.
- Change instrument language during an `info` command.

## [0.19.0]

### Changed

- For instruments that support both SCPI and TSP command languages, force the
  command language to TSP when doing `kic info`

### Added

- Added the `dump` subcommand to connect to an instrument and dump out the contents
  of the output queue.
- Added new `--dump-output` arg to `connect` subcommand to support printing the
  `dump` subcommand details to the terminal

### Fixed

- Fixed CI issue where macOS artifacts would overwrite linux artifacts
- VISA discovery now properly cleans up instrument after getting info

## [0.18.4]

### Fixed

- Fixed issue when getting info from an instrument with data on the output queue

## [0.18.3]

### Added

- Add support for macOS (LAN only)

### Changed

- .script , .upgrade and .exit commands descriptions updated
- When using VISA, only call ReadSTB after writing commands to the instrument
  (or every 3 seconds for a heartbeat)
- Pause when an error occurs so users can see the errors

## [0.18.2]

### Fixed

- Fix issue in discovering instruments when VISA errors

## [0.18.1]

### Fixed

- **tsp-toolkit-kic-lib** Fix issue where versatest instrument fw flash would be
  aborted by drop

## [0.18.0]

### Added

- VISA Support
- Added .reset command to Cancel any ongoing jobs and send *RST.

## [0.17.0]

### Added
- Add reset subcommand to enable quick instrument resetting (TSP-730)
- Added logging infrastructure over socket, in files, and to stderr in `kic` and
  `kic-discover`.

### Fixed
- Fixed an indexing issue for upgrading module firmware (TSP-761) *Open Source Contribution: c3charvat, amcooper181*

## [0.16.2]

### Changed
- Renamed update to upgrade for firmware upgrade in CLI arguments (TSP-741)

## [0.16.1]

### Changed
- Renamed update to upgrade for firmware upgrade (TSP-463)
- **tsp-toolkit-kic-lib** Fix Support for FW flash on the 3706B and 70xB *Open Source Contribution: c3charvat*

## [0.16.0]

### Changed
- Both lxi and usb device info struct's instrument address field has same name (TSP-634)

## [0.15.3]

### Fixed
- Fix issue where unrecognized model number causes kic-cli to never exit (TSP-645)
- Fix issue in which the prompt would be displayed immediately after loading a script

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
- Prepend `kic_` to scripts loaded by `kic_cli` to prevent name-collisions (TSP-505)

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
[Unreleased]: https://github.com/tektronix/tsp-toolkit-kic-cli/compare/v0.21.0...HEAD
[0.21.0]: https://github.com/tektronix/tsp-toolkit-kic-cli/releases/tag/v0.21.0
[0.20.1]: https://github.com/tektronix/tsp-toolkit-kic-cli/releases/tag/v0.20.1
[0.20.0]: https://github.com/tektronix/tsp-toolkit-kic-cli/releases/tag/v0.20.0
[0.19.8]: https://github.com/tektronix/tsp-toolkit-kic-cli/releases/tag/v0.19.8
[0.19.7]: https://github.com/tektronix/tsp-toolkit-kic-cli/releases/tag/v0.19.7
[0.19.6]: https://github.com/tektronix/tsp-toolkit-kic-cli/releases/tag/v0.19.6
[0.19.5]: https://github.com/tektronix/tsp-toolkit-kic-cli/releases/tag/v0.19.5
[0.19.4]: https://github.com/tektronix/tsp-toolkit-kic-cli/releases/tag/v0.19.4
[0.19.3]: https://github.com/tektronix/tsp-toolkit-kic-cli/releases/tag/v0.19.3
[0.19.2]: https://github.com/tektronix/tsp-toolkit-kic-cli/releases/tag/v0.19.2
[0.19.1]: https://github.com/tektronix/tsp-toolkit-kic-cli/releases/tag/v0.19.1
[0.19.0]: https://github.com/tektronix/tsp-toolkit-kic-cli/releases/tag/v0.19.0
[0.18.4]: https://github.com/tektronix/tsp-toolkit-kic-cli/releases/tag/v0.18.4
[0.18.3]: https://github.com/tektronix/tsp-toolkit-kic-cli/releases/tag/v0.18.3
[0.18.2]: https://github.com/tektronix/tsp-toolkit-kic-cli/releases/tag/v0.18.2
[0.18.1]: https://github.com/tektronix/tsp-toolkit-kic-cli/releases/tag/v0.18.1
[0.18.0]: https://github.com/tektronix/tsp-toolkit-kic-cli/releases/tag/v0.18.0
[0.17.0]: https://github.com/tektronix/tsp-toolkit-kic-cli/releases/tag/v0.17.0
[0.16.2]: https://github.com/tektronix/tsp-toolkit-kic-cli/releases/tag/v0.16.2
[0.16.1]: https://github.com/tektronix/tsp-toolkit-kic-cli/releases/tag/v0.16.1
[0.16.0]: https://github.com/tektronix/tsp-toolkit-kic-cli/releases/tag/v0.16.0
[0.15.3]: https://github.com/tektronix/tsp-toolkit-kic-cli/releases/tag/v0.15.3
[0.15.1]: https://github.com/tektronix/tsp-toolkit-kic-cli/releases/tag/v0.15.1
[0.15.0]: https://github.com/tektronix/tsp-toolkit-kic-cli/releases/tag/v0.15.0
[0.14.1]: https://github.com/tektronix/tsp-toolkit-kic-cli/releases/tag/v0.14.1
[0.13.2]: https://github.com/tektronix/tsp-toolkit-kic-cli/releases/tag/v0.13.2
[0.13.0]: https://github.com/tektronix/tsp-toolkit-kic-cli/releases/tag/v0.13.0
[0.12.2]: https://github.com/tektronix/tsp-toolkit-kic-cli/releases/tag/v0.12.2
[0.12.1]: https://github.com/tektronix/tsp-toolkit-kic-cli/releases/tag/v0.12.1
[0.12.0]: https://github.com/tektronix/tsp-toolkit-kic-cli/releases/tag/v0.12.0
[0.11.2]: https://github.com/tektronix/tsp-toolkit-kic-cli/releases/tag/v0.11.2

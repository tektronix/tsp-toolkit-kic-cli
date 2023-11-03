# Change Log

All notable changes to the "teaspoon-comms" extension will be documented in this file.

This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).


<!--
Check [Keep a Changelog](http://keepachangelog.com/) for recommendations on how to structure this file.

    Added -- for new features.
    Changed -- for changes in existing functionality.
    Deprecated -- for soon-to-be removed features.
    Removed -- for now removed features.
    Fixed -- for any bug fixes.
    Security -- in case of vulnerabilities.
-->

<!--
## [Unreleased]
-->
## [v0.9.2]
[v0.9.2 Release Page]

### Changed
- Update firmware for module in slot using new `slot[n].firmware` commands (TSP-383)

## [v0.9.1]
[v0.9.1 Release Page]

### Changed
- Support special characters in .tsp file name

## [v0.8.2]
[v0.8.2 Release Page]

### Changed
- Instrument password is visible now to close kic instances (tsp-224). Tracked by TSP-363

## [v0.8.1]
[v0.8.1 Release Page]

### Changed
- Invalid watchpoint expression data also sent back to UI (TSP-186)

## [v0.8.0]
[v0.8.0 Release Page]

### Changed
- Watchpoint expressions are updated as user adds an expression in Watch pane and also expression are updated for every stack frame. (TSP-271)

### Fixed
- Stack frames information with correct global variables for functions.(TSP-324)

## [v0.7.1]
[v0.7.1 Release Page]

### Changed
- Update flashUtil.tsp script (TSP-329)

## [v0.7.0]
[v0.7.0 Release Page]

### Changed
- User will be able to set expression/value of structured variable e.g table (TSP-311)

### Added
- Firmware update for Versatest (TSP-234)

## [v0.6.2]
[v0.6.2 Release Page]

### Changed
- kiDebugger sends information of structured variable within stacks information (TSP-217)
- User will be able to set values of structured variable (table) (TSP-309)

## [v0.6.1]
[v0.6.1 Release Page]

### Added
- Firmware Update for TTI and 2600-Series Instruments (TSP-235)


## [v0.6.0]
[v0.6.0 Release Page]

### Added
- User will be able to set variable value (TSP-228)


## [v0.5.2]
[v0.5.2 Release Page]

### Fixed
- Synchronized watchpoint evaluation (TSP-199)

## [v0.5.1]
[v0.5.1 Release Page]

### Added
- User will be able to terminate debuggee (TSP-189)

## [v0.5.0]
[v0.5.0 Release Page]

### Added
- Echo socket server to support instrument selection for debugging. (TSP-25)

### Changed
- Handling watchpoint expressions with double quotes (`"`) (TSP-175)

## [v0.4.2]
[v0.4.2 Release Page]

## Added
- Watchpoint API into Rust library. (TSP-152)

### Changed
- Execution of debugger will not break at watchpoint expression update but only on Breakpoint, StepIn, StepOut and StepOver (TSP-175)

## [v0.4.1]
[v0.4.1 Release Page]

### Added
- User will be able to add watchpoint (TSP-158)

## [v0.4.0]
[v0.4.0 Release Page]

### Added
- Step-In API added. (TSP-169)
- Step-Out API added. (TSP-13)
- User can add/remove breakpoint while the debug session in progress. (TSP-171)

## [v0.3.0]
[v0.3.0 Release Page]

### Added
- Automated communication tests (TSP-84)

### Fixed
- Fix issue with password prompt on 2600-series instruments (TSP-141)


## [v0.2.3]
[v0.2.3 Release Page]

### Fixed
- Instrument output is buffered until prompt is seen (TSP-139)
- TSP progress indicator (`>>>>`) fills screen


--------------------------------------------------------------------------------

## [v0.2.2]
[v0.2.2 Release Page]

### Added
- Experimental USBTMC support (TSP-102, TSP-137)

### Changed
- Discovery improvements (TSP-137, TSP-87)

### Fixed
- Prompt not shown after command that does't have output (TSP-134)
- Instrument with `-` in the name is unable to connect (TSP-132)

--------------------------------------------------------------------------------

## [v0.2.1]
[v0.2.1 Release Page]

### Changed
- Added support for TSPop as a model by defaulting to `2606B`(TSP-133)

--------------------------------------------------------------------------------

## [v0.2.0]
[v0.2.0 Release Page]
### Added
- Fully asynchronous LAN communication (TSP-20)
- Change `*LANG` to `TSP` if `*LANG?` returns something not `TSP` (TSP-71)
- Logout of an instrument when exiting (TSP-109)

### Changed
- Change CLI parameters to make room for USB (TSP-107)
- Change special commands to be of the form `.<command>` (e.g. `.help`, `.script`, `.exit`) (TSP-108)
- Save connected instruments to package.json (TSP-111)
- Display discovered instruments as they are found (TSP-111)
- Discovery completed over all interfaces and service names simultaneously (TSP-120)
- Load script by name, enabling multiple scripts to be loaded (TSP-112)

--------------------------------------------------------------------------------

## [v0.1.0]
[v0.1.0 Release Page]
## Added
- Instrument Discover (LAN)
- Instrument Connection (LAN)
    - Send script to instrument (right-click menu within editor or file-explorer)
    - Log in to password protected instrument (password prompt)

--------------------------------------------------------------------------------

[Unreleased]: https://git.keithley.com/trebuchet/teaspoon/ki-comms/-/tree/dev
[v0.9.2]: https://git.keithley.com/trebuchet/teaspoon/ki-comms/-/compare/v(0.9.1)...v0.9.2?from_project_id=33
[v0.9.2 Release Page]: https://git.keithley.com/trebuchet/teaspoon/ki-comms/-/releases/v0.9.2
[v0.9.1]: https://git.keithley.com/trebuchet/teaspoon/ki-comms/-/compare/v(0.8.2)...v0.9.1?from_project_id=33
[v0.9.1 Release Page]: https://git.keithley.com/trebuchet/teaspoon/ki-comms/-/releases/v0.9.1
[v0.8.2]: https://git.keithley.com/trebuchet/teaspoon/ki-comms/-/compare/v(0.8.1)...v0.8.2?from_project_id=33
[v0.8.2 Release Page]: https://git.keithley.com/trebuchet/teaspoon/ki-comms/-/releases/v0.8.2
[v0.8.1]: https://git.keithley.com/trebuchet/teaspoon/ki-comms/-/compare/v(0.8.0)...v0.8.1?from_project_id=33
[v0.8.1 Release Page]: https://git.keithley.com/trebuchet/teaspoon/ki-comms/-/releases/v0.8.1
[v0.8.0]: https://git.keithley.com/trebuchet/teaspoon/ki-comms/-/compare/v(0.7.1)...v0.8.0?from_project_id=33
[v0.8.0 Release Page]: https://git.keithley.com/trebuchet/teaspoon/ki-comms/-/releases/v0.8.0
[v0.7.1]: https://git.keithley.com/trebuchet/teaspoon/ki-comms/-/compare/v(0.7.0)...v0.7.1?from_project_id=33
[v0.7.1 Release Page]: https://git.keithley.com/trebuchet/teaspoon/ki-comms/-/releases/v0.7.1
[v0.7.0]: https://git.keithley.com/trebuchet/teaspoon/ki-comms/-/compare/v0.6.2...v0.7.0?from_project_id=33
[v0.7.0 Release Page]: https://git.keithley.com/trebuchet/teaspoon/ki-comms/-/releases/v0.7.0
[v0.6.2]: https://git.keithley.com/trebuchet/teaspoon/ki-comms/-/compare/v0.6.1...v0.6.2?from_project_id=33
[v0.6.2 Release Page]: https://git.keithley.com/trebuchet/teaspoon/ki-comms/-/releases/v0.6.2
[v0.6.1]: https://git.keithley.com/trebuchet/teaspoon/ki-comms/-/compare/v0.6.0...v0.6.1?from_project_id=33
[v0.6.1 Release Page]: https://git.keithley.com/trebuchet/teaspoon/ki-comms/-/releases/v0.6.1
[v0.6.0]: https://git.keithley.com/trebuchet/teaspoon/ki-comms/-/compare/v0.5.2...v0.6.0?from_project_id=33
[v0.6.0 Release Page]: https://git.keithley.com/trebuchet/teaspoon/ki-comms/-/releases/v0.6.0
[v0.5.2]: https://git.keithley.com/trebuchet/teaspoon/ki-comms/-/compare/v0.5.1...v0.5.2?from_project_id=33
[v0.5.2 Release Page]: https://git.keithley.com/trebuchet/teaspoon/ki-comms/-/releases/v0.5.2
[v0.5.1]: https://git.keithley.com/trebuchet/teaspoon/ki-comms/-/compare/v0.5.0...v0.5.1?from_project_id=33
[v0.5.1 Release Page]: https://git.keithley.com/trebuchet/teaspoon/ki-comms/-/releases/v0.5.1
[v0.5.0]: https://git.keithley.com/trebuchet/teaspoon/ki-comms/-/compare/v0.4.2...v0.5.0?from_project_id=33
[v0.5.0 Release Page]: https://git.keithley.com/trebuchet/teaspoon/ki-comms/-/releases/v0.5.0
[v0.4.2]: https://git.keithley.com/trebuchet/teaspoon/ki-comms/-/compare/v0.4.1...v0.4.2?from_project_id=33
[v0.4.2 Release Page]: https://git.keithley.com/trebuchet/teaspoon/ki-comms/-/releases/v0.4.2
[v0.4.1]: https://git.keithley.com/trebuchet/teaspoon/ki-comms/-/compare/v0.4.0...v0.4.1?from_project_id=33
[v0.4.1 Release Page]: https://git.keithley.com/trebuchet/teaspoon/ki-comms/-/releases/v0.4.1
[v0.4.0]: https://git.keithley.com/trebuchet/teaspoon/ki-comms/-/compare/v0.3.0...v0.4.0?from_project_id=33
[v0.4.0 Release Page]: https://git.keithley.com/trebuchet/teaspoon/ki-comms/-/releases/v0.4.0
[v0.3.0]: https://git.keithley.com/trebuchet/teaspoon/ki-comms/-/compare/v0.2.2...v0.3.0?from_project_id=33
[v0.3.0 Release Page]: https://git.keithley.com/trebuchet/teaspoon/ki-comms/-/releases/v0.3.0
[v0.2.3]: https://git.keithley.com/trebuchet/teaspoon/ki-comms/-/compare/v0.2.2...v0.2.3?from_project_id=33
[v0.2.3 Release Page]: https://git.keithley.com/trebuchet/teaspoon/ki-comms/-/releases/v0.2.3
[v0.2.2]: https://git.keithley.com/trebuchet/teaspoon/ki-comms/-/compare/v0.2.1...v0.2.2?from_project_id=33
[v0.2.2 Release Page]: https://git.keithley.com/trebuchet/teaspoon/ki-comms/-/releases/v0.2.2
[v0.2.1]: https://git.keithley.com/trebuchet/teaspoon/ki-comms/-/compare/v0.2.0...v0.2.1?from_project_id=33
[v0.2.1 Release Page]: https://git.keithley.com/trebuchet/teaspoon/ki-comms/-/releases/v0.2.1
[v0.2.0]: https://git.keithley.com/trebuchet/teaspoon/ki-comms/-/compare/v0.1.0...v0.2.0?from_project_id=33
[v0.2.0 Release Page]: https://git.keithley.com/trebuchet/teaspoon/ki-comms/-/releases/v0.2.0
[v0.1.0]: https://git.keithley.com/trebuchet/teaspoon/ki-comms/-/tree/v0.1.0
[v0.1.0 Release Page]: https://git.keithley.com/trebuchet/teaspoon/ki-comms/-/releases/v0.1.0

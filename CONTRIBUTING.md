# Contribution Guide

## Dependencies

In order to contribute code to this project you must have the following dependencies installed:

* Latest NodeJS LTS
* Latest Nightly Rust toolchain with the following components
    * `rustc` (default)
    * `rust-std` (default)
    * `cargo` (default)
    * `rust-docs` (default)
    * `rustfmt` (default)
    * `clippy` (default)
    * `llvm-tools` (required for code-coverage)
    * `rust-analyzer` (optional, Not needed if using VSCode Rust Analyzer Extension)
    * `rust-src` (optional)
* `grcov` (required for code-coverage)
    * `cargo install grcov`
* `cargo2junit` (required for code-coverage)
    * `cargo install cargo2junit`

### To check
* Go to (NodeJS)[https://nodejs.org] and check the latest LTS version
    * Run `node --version` from the command line and ensure your local version matches.
    * If not, install the latest version.
* Run `rustup update` to get the latest rust toolchain.

## Getting Started

0. Ensure [all dependencies](#dependencies) are installed
1. Clone this repository locally: `git clone git@github.com:tektronix/tsp-toolkit-kic-cli.git`
2. Change to the root project directory: `cd tsp-toolkit-kic-cli`
3. Run tests to ensure everything is set up properly: `cargo test`
4. Install dependencies as required

## Code Quality

To ensure your code quality meets our standards before opening a pull request, please
run the following commands:

```bash
cargo fmt
cargo clippy --tests
# warnings *may* be acceptable, but please clean up if at all possible
```
## Building the Application

In order for a PR to be accepted, there must be no errors or warnings when compiling
this repo in debug or release mode.

```bash
# build with binary generation
cargo build
cargo build --release

# Run compiler without binary generation (faster)
cargo check
cargo check --release
```

## Testing

To ensure that our code remains functional and avoids regressions, please ensure all
tests pass before opening a pull request.

```bash
cargo test
```

If a test fails due to changed functionality, please double check to make sure that
functionality is expected and then correct the test to align with the new logic.

### Test Coverage
To ensure that our code resists regressions, the unit-test code-coverage percentage
must remain the same or better. You can run the following commands locally to check
the code coverage.

After you perform the steps for your OS, you can look for the coverage information in
the resultant `target/coverage/markdown.md` file.

#### Windows

```powershell
$env:RUSTFLAGS='-Cinstrument-coverage'
$env:COVERAGE_DIR='target/coverage'
$env:LLVM_PROFILE_FILE="../${env:COVERAGE_DIR}/%p-%m.profraw"

mkdir -p "${env:COVERAGE_DIR}"
cargo test --workspace -- -Z unstable-options --format json --report-time | cargo2junit > report.xml
grcov "${env:COVERAGE_DIR}" --binary-path "target/debug" -s . -o "${env:COVERAGE_DIR}" --ignore-not-existing --ignore '**/.cargo/*' --output-types markdown

$env:RUSTFLAGS=$null
$env:COVERAGE_DIR=$null
$env:LLVM_PROFILE_FILE=$null

```

#### Linux/macOS

```bash
RUSTFLAGS='-Cinstrument-coverage'
COVERAGE_DIR='target/coverage'
LLVM_PROFILE_FILE="../${COVERAGE_DIR}/%p-%m.profraw"
mkdir "${COVERAGE_DIR}"
cargo test --workspace -- -Z unstable-options --format json --report-time | cargo2junit > report.xml
grcov ${COVERAGE_DIR} --binary-path target/debug -s . -o "${COVERAGE_DIR}" --ignore-not-existing --ignore '**/.cargo/*' --output-types markdown

```

## Formatting

All code must conform to `rustfmt` formatting using the defaults plus whatever might be
configured for this workspace. It is recommended that you configure your editor to
format whenever you save.

```bash
# check formatting, output issues
cargo fmt --check --verbose

# correct formatting, write to files
cargo fmt
```

## Linting

All code must pass linting with no warnings or errors. If exceptions must be made,
make sure a comment explaining the exception is present next to the `#[allow(...)]`
statement. These excpetions *may* be revoked during a code review to ensure ongoing
code quality.

```bash
cargo clippy

# lint including tests
cargo clippy --tests
```

## Contributing/Submitting Changes

Create a pull request against the `dev` branch. Be sure to include a reference to the
issue you are closing as well as details of what you changed.

## Other Important Commands

### Find tool versions

```bash
# rust tools
rustc --version
cargo --version
cargo fmt --version
cargo clippy --version

# NodeJS tools
npm --version
node --version

# Other tools
grcov --version

```


### Generate Software Bill of Materials (SBOM)

You will not need to generate a CycloneDX SBOM in order to contribute to this project.
If you *want* to generate an SBOM, here is the command used by CI.

In order to run these commands, you will need to have `cargo-cyclonedx` installed.

```bash
cargo cyclonedx --all --output-cdx --format json
```

If you don't have this tool installed, you can run `cargo install cargo-cyclonedx`



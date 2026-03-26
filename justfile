#!/usr/bin/env just --justfile

set windows-shell := ['powershell']

vendor := if os() == 'windows' { "-pc" } else if os() == 'darwin' { "-apple" } else { "-unknown" }
build := if os() == 'windows' { "-msvc" } else if os() == 'darwin' { "" } else { "-gnu" }
vsc-os := if os() == 'windows' { "win32" } else { os() }
vsc-arch := if arch() == "x86_64" { "x64" } else { "arm64" }
exe-extension := if os() == 'windows' { ".exe" } else { "" }
package-name := replace_regex(`npm pkg get name`, "\"", "")
orig-package-os := replace_regex(`npm pkg get os`, "\\{}", "")
orig-package-cpu := replace_regex(`npm pkg get cpu`, "\\{}", "")
native-triple := (arch() + vendor + "-" + os() + build)
native-vscode-platform := (vsc-os + "-" + vsc-arch)

# List all possible targets
default triple=native-triple:
    @echo "{{ package-name }}"
    @just --list

# Prepare for a PR by initializing, formatting, linting, building, testing, and packaging the project
pr triple=native-triple: init fmt lint (build triple) test (package triple)

# Initialize all tooling
init: init-rust init-root

# Check code formatting without making changes
check-fmt: check-fmt-rust

clean: clean-sbom clean-rust clean-root

# Format code
fmt: fmt-rust

# Lint code
lint: lint-rust

# Build code for a given architecture or the current architecture, if none is provided
build triple=native-triple: (build-rust triple)

# Build the release version for the given architecture or the current architecture, if none is provided
build-release triple=native-triple: (build-release-rust triple)

# Run all tests
test: test-rust

# Run tests with code-coverage enabled
test-cov: test-cov-rust

# Generate the SBOM(s)
sbom: sbom-rust

# Clean up the target directory before packaging
pre-package triple=native-triple target-dir="": (pre-package-rust triple)
    {{ if target-dir != "" { "rm -r " + target-dir + "/bin" + "/*" } else { "" } }}
    {{ if target-dir != "" { "cp -r bin/* '" + target-dir + "/bin'" } else { "" } }}

# Set the packaging details and then make a package, then cleanup (reset the packaging details)
package vscode-platform=native-vscode-platform os=os() cpu=arch() triple=native-triple: && packaging-cleanup
    npm pkg set "name={{ package-name }}-{{ vscode-platform }}" --verbose
    npm pkg set "os[0]={{ os }}" --verbose
    npm pkg set "cpu[0]={{ cpu }}" --verbose
    cat package.json
    npm pack

# Cleanup the changes made to package information
[private]
packaging-cleanup:
    npm pkg set "name={{ package-name }}" --verbose
    npm pkg {{ if orig-package-os != "" { 'set "os[0]={{orig-package-os}}"' } else { 'delete "os"' } }} --verbose
    npm pkg {{ if orig-package-cpu != "" { 'set "cpu[0]={{orig-package-cpu}}"' } else { 'delete "cpu"' } }} --verbose

################################################################################
# INIT #########################################################################
################################################################################

# Initialize the root npm project
[group("init")]
init-root:
    npm install --devDependencies

# Initialize all rust projects
[group("init")]
[group("rust")]
init-rust:
    cargo check

################################################################################
# CHECK-FMT ####################################################################
################################################################################

# Check the formatting of the Rust projects without making changes
[group("check-fmt")]
[group("rust")]
check-fmt-rust:
    cargo fmt --check

################################################################################
# CLEAN ########################################################################
################################################################################

# Clean up SBOM files
[group("clean")]
[group("rust")]
clean-sbom:
    -rm -r {{ env("SBOM_DIR", "sbom") }}
    -rm instrument-repl/*.cdx.*
    -rm kic/*.cdx.*
    -rm kic-debug/*.cdx.*
    -rm kic-debug-visa/*.cdx.*
    -rm kic-discover/*.cdx.*
    -rm kic-discover-visa/*.cdx.*
    -rm kic-lib/*.cdx.*

# Clean up Rust files
[group("clean")]
[group("rust")]
clean-rust:
    cargo clean

# Clean up root project files (mostly npm)
[group("clean")]
[group("rust")]
clean-root:
    -rm -r node_modules

################################################################################
# FMT ##########################################################################
################################################################################

# Format Rust code
[group("fmt")]
[group("rust")]
fmt-rust:
    cargo fmt

################################################################################
# LINT #########################################################################
################################################################################

# Lint Rust code
[group("lint")]
[group("rust")]
lint-rust: init-root
    cargo clippy
    cargo clippy --tests

################################################################################
# BUILD ########################################################################
################################################################################

# Build kic
[group("build")]
[group("rust")]
build-rust triple=native-triple: (build-visa triple) (build-non-visa triple)

[parallel]
[private]
build-visa triple=native-triple release="": (build-kic-visa triple release) (build-kic-debug-visa triple release) (build-kic-discover-visa triple release)

[parallel]
[private]
build-non-visa triple=native-triple release="": (build-kic triple release) (build-kic-debug triple release) (build-kic-discover triple release)

[private]
build-kic-visa triple=native-triple release="":
    -rm target/{{ triple }}/debug/kic-visa{{ exe-extension }}
    cargo build -p kic -F visa --target {{ triple }} {{ release }}
    mv target/{{ triple }}/debug/kic{{ exe-extension }} target/{{ triple }}/debug/kic-visa{{ exe-extension }}

[private]
build-kic triple=native-triple release="":
    cargo build -p kic --target {{ triple }} {{ release }}

[private]
build-kic-debug triple=native-triple release="":
    cargo build -p kic-debug --target {{ triple }} {{ release }}

[private]
build-kic-debug-visa triple=native-triple release="":
    cargo build -p kic-debug-visa --target {{ triple }} {{ release }}

[private]
build-kic-discover triple=native-triple release="":
    cargo build -p kic-discover --target {{ triple }} {{ release }}

[private]
build-kic-discover-visa triple=native-triple release="":
    cargo build -p kic-discover-visa --target {{ triple }} {{ release }}

################################################################################
# BUILD-RELEASE ################################################################
################################################################################

# Build Rust code in release mode
[group("build-release")]
[group("rust")]
build-release-rust triple=native-triple: (build-visa triple "--release") (build-non-visa triple "--release")

################################################################################
# TEST #########################################################################
################################################################################

# Run all Rust tests
[group("rust")]
[group("test")]
test-rust:
    -rm -r "{{ env("TEST_DIR", "test-results") }}"
    -mkdir -p '{{ env("TEST_DIR", "test-results") }}'
    cargo nextest r --all --all-targets
    @mv test-results/* "{{ env("TEST_DIR", "test-results") }}"

################################################################################
# TEST-COV #####################################################################
################################################################################

# Run all Rust tests with code coverage enabled
[group("rust")]
[group("test-cov")]
test-cov-rust $CARGO_TERM_VERBOSE="true":
    -rm -r "{{ env("TEST_DIR", "test-results") }}"
    -mkdir -p '{{ env("TEST_DIR", "test-results") }}'
    cargo llvm-cov nextest --cobertura --branch > "{{ env("TEST_DIR", "test-results") }}/kic-rust.cobertura.xml"
    @mv test-results/* "{{ env("TEST_DIR", "test-results") }}"

################################################################################
# SBOM #########################################################################
################################################################################

# Generate Rust Software Bill of Materials (SBOM) and place in SBOM_DIR or sbom/
[group("rust")]
[group("sbom")]
sbom-rust:
    -rm -r {{ env("SBOM_DIR", "sbom") }}
    -mkdir {{ env("SBOM_DIR", "sbom") }}
    cargo cyclonedx --format json --all --describe crate -vvv
    mv instrument-repl/*.cdx.json {{ env("SBOM_DIR", "sbom") }}
    mv kic/*.cdx.json {{ env("SBOM_DIR", "sbom") }}
    mv kic-debug/*.cdx.json {{ env("SBOM_DIR", "sbom") }}
    mv kic-debug-visa/*.cdx.json {{ env("SBOM_DIR", "sbom") }}
    mv kic-discover/*.cdx.json {{ env("SBOM_DIR", "sbom") }}
    mv kic-discover-visa/*.cdx.json {{ env("SBOM_DIR", "sbom") }}
    mv kic-lib/*.cdx.json {{ env("SBOM_DIR", "sbom") }}

################################################################################
# PACKAGE ######################################################################
################################################################################

# Delete the bin dir and make a new one
[private]
prep-package:
    -rm -r bin
    -mkdir -p bin

# BUILD MUST BE RUN FIRST! move all executables to bin directory
[group("package")]
[group("rust")]
pre-package-rust triple=native-triple: prep-package
    cp target/{{ triple }}/release/kic-* ./bin
    -rm bin/*.pdb
    -rm bin/*.d

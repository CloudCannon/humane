# Changelog

<!-- 
    Add changes to the Unreleased section during development.
    Do not change this header — the GitHub action that releases
    this project will edit this file and add the version header for you.
    The Unreleased block will also be used for the GitHub release notes.
-->

## Unreleased

## v0.9.1 (May 7, 2024)

* Halved Humane's baked in concurrency
  * Resolves some CI issues
  * This is a temporary measure while Humane's rework is underway

## v0.9.0 (June 9, 2023)

* Add `@platform-*` tags that can target scenarios to `linux`, `macos`, `unix`, or `windows`

## v0.8.0 (April 13, 2023)

* Run substitutions when writing files

## v0.7.0 (April 13, 2023)

* Add `I run "command"` step definition
* Add `{{humane_cwd}}` substitution to commands

## v0.6.0 (November 12, 2022)

* Add {{humane_temp_dir}} substitution in program flags

## v0.5.0 (November 5, 2022)

* Support writing gzipped files to disk

## v0.4.4 (October 26, 2022)

* No changes. Testing release workflows.

## v0.4.3 (October 25, 2022)

* No changes. Testing release workflows.

## v0.4.0 (October 13, 2022)

* Release on M1

## v0.3.29 (October 12, 2022)

* Fix indentation underflow in test failure error message when a file doesn't exist

## v0.3.28 (October 12, 2022)

* No changes. Testing release workflows.

## v0.3.27 (October 12, 2022)

* No changes. Testing release workflows.

## v0.3.25 (October 12, 2022)

* No changes. Testing release workflows.

## v0.3.17 (October 12, 2022)

* No changes. Testing release workflows.

## v0.3.12 (September 22, 2022)

* Fix: Exit NPM wrapper with an error code if tests fail

## v0.3.11 (September 22, 2022)

* Fix: Correctly exit with an error code if tests fail

## v0.3.10 (September 20, 2022)

* Fix: Exit with an error code if tests fail

## v0.3.9 (September 14, 2022)

* Support checking bools in humane JSON tests

## v0.3.8 (September 12, 2022)

* Add stdout/stderr selection support for testing strings

## v0.3.7 (August 10, 2022)

* Add an `en` lang to the default html page template

## v0.3.6 (August 9, 2022)

* Fix to not require the `--name` flag

## v0.3.5 (August 9, 2022)

* Re-exported the `-n` / `--name` CLI flag from cucumber to allow running individual tests

## v0.3.4 (August 4, 2022)

* Fixed the uploading of release binaries via CI.

## v0.3.3 (August 4, 2022) [BROKEN]

* Implemented changelog handling and release notes as part of the automated release flow.

## v0.3.2 (August 4, 2022)

* Sorted out a Humane publishing flow for Windows.
* Improved the atomicity of the automated release flow.
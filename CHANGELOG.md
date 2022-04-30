# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres
to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Changed

* Don't treat adding the same version as an error for the `add` command.
* Update Proton apply hint for `--apply` argument and `apply` command to be more accurate/helpful.

### Fixed

* Create required Steam or Lutris directories if missing. This effects the following directories:
  * `XDG_DATA_HOME/Steam/compatibilitytools.d`
  * `XDG_DATA_HOME/lutris/runners/wine`
  * `XDG_CONFIG_HOME/lutris/runners`
* Create global Wine runner config `XDG_CONFIG_HOME/lutris/runners/wine.yml` for Lutris if not present.
* Use `.steam` folder in user home directory for Steam path operations. Steam is not installed under the XDG
  standard on all distributions.
* Do not download the latest version when that version is already managed by GE-Man. This effects `add` commands
  with no specific tag version provided.

## [0.1.0] - 2022-03-27

### Added

* "add" command to download GE Proton or Wine GE versions.
* "remove" command to delete a downloaded GE Proton or Wine GE version.
* "check" command to show the latest release tags available for a GE Proton or Wine GE version.
* "migrate" command to make an existing GE Proton or Wine GE version on a hard-drive manageable by GE Helper.
* "forget" command to remove a version from GE Helper without deleting it.
* "list" command to list all versions managed by GE Helper.
* "user-settings copy" command to copy a user-settings.py file from one GE Proton version to another.
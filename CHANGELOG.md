# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres
to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Removed

* The `forget` command has been removed. In replacement of it the `remove` and `clean` command support
  a `--forget` (`-f`) flag.<br>
  The motivation behind this change is that `forget` by itself does too little to merit its existence as a seperate
  command.

### Added

* Add the ability to set the Steam installation path via a config file in `$XDG_CONFIG_HOME/ge_man/config.json`. <br>
  Alternatively, the Steam path can also be set with the `GE_MAN_STEAM_PATH` variable.
* `list` command
  * Added the `--file-system` (`-f`) flag<br>
    When this flag is set the content of the Steam compatibilitytools.d or Lutris runners folder is listed. This is
    helpful for migrating folders or just checking in general what else is present in those directories. Bear in
    mind that this flag will display both GE-Man managed and non-managed versions!
* `clean` command
  * Remove multiple GE Proton or Wine GE versions.
  * The `--before` (`-b`) flag can be used to remove all versions before a given version.
  * The `--start` (`-s`) and `--end` (`-e`) flags can be used to remove a range of versions.<br>
    The start and end versions are excluded from removal.
  * The `--forget` (`-f`) flag can be used to forget a version in GE-Man.

### Changed

* Show the file name of the to-be-downloaded archive during download.

* Technical:
  * Renamed `TerminalWriter` to `CommandHandler` to better represent its purpose
  * Renamed to `ui` module to `command_execution`
  * Renamed the `args` module to `command_input`
  * Refactored the interaction with the `CommandHandler` to be more input/output focused

## [0.1.2] - 2022-06-17

### Changed

* Bring dependencies up-to-date

## [0.1.1] - 2022-04-30

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
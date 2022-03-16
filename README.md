# GE Helper

GE Helper is a version manager for GE Proton and Wine GE versions.

Currently, it has the following features:

* Manage Proton GE or Wine GE versions
  * Download versions by their Github release tag
  * Remove downloaded versions
  * Set the default Proton version for Steam
  * Set the default Wine version for Lutris
* Copy a Proton user-settings file from one version to another

And these features are planned:

* Add labels to a version
* Apply a version by manually specifying its directory name
* Remove versions that are before release X
* More functionality for the user-settings command
  * Apply a user-settings.py file to a version
  * Add a user-settings.py file to gehelper to make it applicable.

# Installation

GE Helper can be installed with cargo or by downloading the precompiled binaries from the release page.

`cargo install ge-helper`

When installing with cargo the resulting binary is placed into `$HOME/.cargo`. To make the binary accessible from
everywhere in a terminal add the `$HOME/.cargo` path to the `PATH` environment variable.

# Usage

GE Helper provides the following commands:

* `add` - Add a GE Proton or Wine GE version
* `remove` (`rm`) - Remove a GE Proton version or Wine GE version
* `check` (`ck`) - Display the latest GE Proton, Wine GE and Wine GE LoL version
* `apply` - Set the default compatibility tool for Steam or Lutris
* `list` - List versions managed by gehelper
* `migrate` - (`mg`) - Make an existing GE version manageable by gehelper
* `user-settings` (`us`) - Commands that relate to Proton user-settings.py files
  * `copy` - Copy a user-settings.py file from on Proton version to another

Every command supports a `--help` argument to view possible parameters and general usage information.

## How do I add a new version?

```sh
# Proton GE
gehelper add -p GE-Proton7-8

# Wine GE
gehelper add -w GE-Proton7-6

# Wine GE for LoL
gehelper add -l 7.0-GE-1-LoL
```

You can also directly apply the downloaded version by using the `--update-config` option.<br>
If no release is provided to the `-p`, `-w` and `-l` options, the latest release is downloaded.

## How do I remove a version?

```sh
# Proton GE
gehelper rm -p GE-Proton7-8

# Wine GE
gehelper rm -w GE-Proton7-6

# Wine GE for LoL
gehelper rm -l 7.0-GE-1-LoL
```

This operation will delete the versions file from the hard drive. If you wish to keep the files and only "forget"
the version in gehelper then use the `forget` command.

## How can I view the latest releases?

```sh
# All GE kinds
gehelper check

# Proton GE
gehelper check -p

# Wine GE
gehelper check -w

# Wine GE for LoL
gehelper check -l
```

## How can I remove a version without deleting its files?

```sh
# Proton GE
gehelper forget -p GE-Proton7-8

# Wine GE
gehelper forget -w GE-Proton7-6

# Wine GE for LoL
gehelper forget -l 7.0-GE-1-LoL
```

## How can I list the gehelper managed versions?

```sh
# All GE kinds
gehelper list

# Proton GE
gehelper list -p

# Wine GE
gehelper list -w

# Wine GE for LoL
gehelper list -l
```

## How can I make my existing GE versions manageable by gehelper?

To make an existing version manageable by gehelper you need to use the `migrate` command. The `migrate` command takes a
path to a directory containing a GE version and the kind of GE version.

```sh
# Proton GE
gehelper migrate -s $HOME/.local/share/Steam/compatibilitytools.d/GE-Proton7-8 -p GE-Proton7-8

# Wine GE
gehelper migrate -s $HOME/.local/share/lutris/runners/wine/lutris-GE-Proton7-6-x86_64/ -w GE-Proton7-6

# Wine GE for LoL
gehelper migrate -s $HOME/.local/share/lutris/runners/wine/lutris-ge-7.0-1-lol-x86_64 -l 7.0-GE-1-LoL
```

use clap::{command, crate_version, Arg, ArgGroup, Command};

pub const APP_NAME: &str = "GE Helper";

pub mod command_names {
    pub const LIST: &str = "list";
    pub const ADD: &str = "add";
    pub const REMOVE: &str = "remove";
    pub const CHECK: &str = "check";
    pub const MIGRATE: &str = "migrate";
    pub const APPLY: &str = "apply";
    pub const PROTON_USER_SETTINGS: &str = "user-settings";
    pub const USER_SETTINGS_COPY: &str = "copy";
    pub const CLEAN: &str = "clean";
}

pub mod arg_names {
    pub const LOL_ARG: &str = "lol";
    pub const WINE_ARG: &str = "wine";
    pub const PROTON_ARG: &str = "proton";
    pub const NEWEST_ARG: &str = "newest";
    pub const SKIP_CHECKSUM_ARG: &str = "skip-checksum";
    pub const APPLY_ARG: &str = "apply";
    pub const SOURCE_ARG: &str = "source";
    pub const DESTINATION_ARG: &str = "destination";
    pub const FILE_SYSTEM: &str = "file-system";
    pub const BEFORE: &str = "before";
    pub const START: &str = "start";
    pub const END: &str = "end";
    pub const FORGET: &str = "forget";
    pub const DRY_RUN: &str = "dry-run";
}

pub mod arg_group_names {
    pub const TAG: &str = "tag-args";
    pub const START_END_RANGE: &str = "start-end-range";
    pub const BEFORE_START_END: &str = "before-start-end";
}

pub mod about_text {
    pub const ADD: &str =
        "Add a GE Proton version for Steam or a Wine GE version for Lutris. If no <TAG> value is provided, the \
         latest version is assumed.";
    pub const LIST: &str = "List available versions.";
    pub const REMOVE: &str =
        r#"Remove a GE Proton version for Steam or a Wine GE version for Lutris. "remove" is aliased to "rm""#;
    pub const CHECK: &str = r#"Check for the latest Github release tags. "check" is aliased to "ck"."#;
    pub const MIGRATE: &str =
        r#"Migrate an existing GE Proton/Wine GE version to make it manageable. "migrate" is aliased to "mg""#;
    pub const APPLY: &str =
        "Apply a Proton GE or Wine GE version by updating the used version in the Steam or Lutris config files.";
    pub const USER_SETTINGS: &str =
        r#"Commands for managing user setting files for Proton versions. "user-settings" is aliased to "us""#;
    pub const USER_SETTINGS_COPY: &str = "Copy a user_settings.py from one Proton version to another.";
    pub const FORGET: &str = "Forget a GE Proton or Wine GE version. This operation does not remove any files.";
    pub const CLEAN: &str = "Remove multiple GE Proton or Wine GE versions.";
}

mod help_text {
    // Add
    pub const ADD_PROTON_TAG: &str = "Download a GE Proton version";
    pub const ADD_WINE_TAG: &str = "Download a Wine GE version";
    pub const ADD_WINE_LOL_TAG: &str = "Download a Wine GE LoL version";
    pub const ADD_SKIP_CHECKSUM: &str = "Skip downloading the checksum and comparing it to the downloaded release.";
    pub const ADD_APPLY: &str = "Set the Steam or Lutris compatibility tool version after successfully adding it.";
    // List
    pub const LIST_PROTON_TAG: &str = "List GE Proton versions";
    pub const LIST_WINE_TAG: &str = "List Wine GE versions";
    pub const LIST_WINE_LOL_TAG: &str = "List Wine GE LoL versions";
    pub const LIST_NEWEST: &str = "List the latest versions for each tag kind.";
    pub const LIST_FILE_SYSTEM: &str = "List content of Steam compatibility tools or Lutris runners folder";
    // Remove
    pub const REMOVE_PROTON_TAG: &str = "Remove a GE Proton version";
    pub const REMOVE_WINE_TAG: &str = "Remove a Wine GE version";
    pub const REMOVE_WINE_LOL_TAG: &str = "Remove a Wine GE LoL version";
    pub const REMOVE_FORGET: &str = "Do not remove the versions from the file system, only forget them in GE-Man";
    // Check
    pub const CHECK_PROTON_TAG: &str = "Check for the latest GE Proton version";
    pub const CHECK_WINE_TAG: &str = "Check for the latest Wine GE version";
    pub const CHECK_WINE_LOL_TAG: &str = "Check for the latest Wine GE LoL version";
    // Migrate
    pub const MIGRATE_PROTON_TAG: &str = "Migrate a GE Proton version";
    pub const MIGRATE_WINE_TAG: &str = "Migrate a Wine GE version";
    pub const MIGRATE_WINE_LOL_TAG: &str = "Migrate a Wine GE LoL version";
    pub const MIGRATE_SOURCE: &str = "Path to a directory containing a Wine GE or Proton GE version.";
    // Apply
    pub const APPLY_PROTON_TAG: &str = "Apply a GE Proton version for Steam";
    pub const APPLY_WINE_TAG: &str = "Apply a Wine GE version for Lutris";
    pub const APPLY_WINE_LOL_TAG: &str = "Apply a Wine GE LoL version for Lutris";
    // User settings copy
    pub const USER_SETTINGS_COPY_SOURCE: &str = "Source tag where to copy the user_settings.py from.";
    pub const USER_SETTINGS_COPY_DESTINATION: &str = "Destination tag where to copy the user_settings.py to.";
    // Clean
    pub const CLEAN_PROTON_TAG: &str = "Consider following tags as GE Proton tags";
    pub const CLEAN_WINE_TAG: &str = "Consider following tags as Wine GE tags";
    pub const CLEAN_WINE_LOL_TAG: &str = "Consider following tags as Wine GE (LoL) tags";
    pub const CLEAN_BEFORE_TAG: &str = "Remove all versions before this tag - This argument conflicts with start and \
    end";
    pub const CLEAN_START_TAG: &str = "Sets the start tag for range removal";
    pub const CLEAN_END_TAG: &str = "Sets the end tag for range removal";
    pub const CLEAN_FORGET: &str = "Do not remove the versions from the file system, only forget them in GE-Man";
    pub const CLEAN_DRY_RUN: &str = "Do not perform any actions, only show affected versions";
}

pub mod value_names {
    pub const TAG: &str = "TAG";
    pub const PATH: &str = "PATH";
}

mod group {
    use super::*;

    pub fn start_end_range(required: bool, conflicts: &[&'static str]) -> ArgGroup<'static> {
        ArgGroup::new(arg_group_names::START_END_RANGE)
            .args(&[arg_names::START, arg_names::END])
            .required(required)
            .multiple(true)
            .requires_all(&[arg_names::START, arg_names::END])
            .conflicts_with_all(conflicts)
    }

    pub fn before_and_start_end(required: bool) -> ArgGroup<'static> {
        ArgGroup::new(arg_group_names::BEFORE_START_END)
            .args(&[arg_names::START, arg_names::END, arg_names::BEFORE])
            .required(required)
            .multiple(true)
    }

    pub fn tag_args(required: bool) -> ArgGroup<'static> {
        ArgGroup::new(arg_group_names::TAG)
            .args(&[arg_names::PROTON_ARG, arg_names::WINE_ARG, arg_names::LOL_ARG])
            .required(required)
    }
}

mod arg {
    use super::*;

    pub mod proton {
        use super::*;

        pub fn flag(help_text: &'static str, display_order: usize) -> Arg {
            Arg::new(arg_names::PROTON_ARG)
                .short('p')
                .long(arg_names::PROTON_ARG)
                .help(help_text)
                .display_order(display_order)
                .takes_value(false)
        }

        pub fn with_value(help_text: &'static str, min_value: usize, display_order: usize) -> Arg {
            Arg::new(arg_names::PROTON_ARG)
                .short('p')
                .long(arg_names::PROTON_ARG)
                .help(help_text)
                .display_order(display_order)
                .takes_value(true)
                .value_name(value_names::TAG)
                .min_values(min_value)
                .max_values(1)
                .multiple_values(false)
        }
    }

    pub mod wine {
        use super::*;

        pub fn flag(help_text: &'static str, display_order: usize) -> Arg {
            Arg::new(arg_names::WINE_ARG)
                .short('w')
                .long(arg_names::WINE_ARG)
                .help(help_text)
                .display_order(display_order)
                .takes_value(false)
        }

        pub fn with_value(help_text: &'static str, min_value: usize, display_order: usize) -> Arg {
            Arg::new(arg_names::WINE_ARG)
                .short('w')
                .long(arg_names::WINE_ARG)
                .help(help_text)
                .display_order(display_order)
                .takes_value(true)
                .value_name(value_names::TAG)
                .min_values(min_value)
                .max_values(1)
                .multiple_values(false)
        }
    }

    pub mod lol {
        use super::*;

        pub fn flag(help_text: &'static str, display_order: usize) -> Arg {
            Arg::new(arg_names::LOL_ARG)
                .short('l')
                .long(arg_names::LOL_ARG)
                .help(help_text)
                .display_order(display_order)
                .takes_value(false)
        }

        pub fn with_value(help_text: &'static str, min_value: usize, display_order: usize) -> Arg {
            Arg::new(arg_names::LOL_ARG)
                .short('l')
                .long(arg_names::LOL_ARG)
                .help(help_text)
                .display_order(display_order)
                .takes_value(true)
                .value_name(value_names::TAG)
                .min_values(min_value)
                .max_values(1)
                .multiple_values(false)
        }
    }

    pub fn newest(help_text: &'static str, display_order: usize) -> Arg {
        Arg::new(arg_names::NEWEST_ARG)
            .short('n')
            .long(arg_names::NEWEST_ARG)
            .display_order(display_order)
            .help(help_text)
    }

    pub fn skip_checksum(help_text: &'static str, display_order: usize) -> Arg {
        Arg::new(arg_names::SKIP_CHECKSUM_ARG)
            .long(arg_names::SKIP_CHECKSUM_ARG)
            .display_order(display_order)
            .help(help_text)
    }

    pub fn apply(help_text: &'static str, display_order: usize) -> Arg {
        Arg::new(arg_names::APPLY_ARG)
            .long(arg_names::APPLY_ARG)
            .display_order(display_order)
            .help(help_text)
    }

    pub fn file_system(help_text: &'static str, display_order: usize) -> Arg {
        Arg::new(arg_names::FILE_SYSTEM)
            .short('f')
            .long(arg_names::FILE_SYSTEM)
            .display_order(display_order)
            .help(help_text)
    }

    pub fn before(help_text: &'static str, display_order: usize) -> Arg {
        Arg::new(arg_names::BEFORE)
            .short('b')
            .long(arg_names::BEFORE)
            .display_order(display_order)
            .help(help_text)
            .takes_value(true)
            .value_name(value_names::TAG)
            .min_values(1)
            .max_values(1)
            .multiple_values(false)
    }

    pub fn start(help_text: &'static str, display_order: usize) -> Arg {
        Arg::new(arg_names::START)
            .short('s')
            .long(arg_names::START)
            .display_order(display_order)
            .help(help_text)
            .takes_value(true)
            .value_name(value_names::TAG)
            .min_values(1)
            .max_values(1)
            .multiple_values(false)
    }

    pub fn end(help_text: &'static str, display_order: usize) -> Arg {
        Arg::new(arg_names::END)
            .short('e')
            .long(arg_names::END)
            .display_order(display_order)
            .help(help_text)
            .takes_value(true)
            .value_name(value_names::TAG)
            .min_values(1)
            .max_values(1)
            .multiple_values(false)
    }

    pub fn forget(help_text: &'static str, display_order: usize) -> Arg {
        Arg::new(arg_names::FORGET)
            .short('f')
            .long(arg_names::FORGET)
            .display_order(display_order)
            .help(help_text)
    }

    pub fn dry_run(help_text: &'static str, display_order: usize) -> Arg {
        Arg::new(arg_names::DRY_RUN)
            .long(arg_names::DRY_RUN)
            .display_order(display_order)
            .help(help_text)
    }

    pub fn source(help_text: &'static str, display_order: usize) -> Arg {
        Arg::new(arg_names::SOURCE_ARG)
            .short('s')
            .long(arg_names::SOURCE_ARG)
            .help(help_text)
            .takes_value(true)
            .required(true)
            .display_order(display_order)
            .value_name(value_names::TAG)
    }

    pub fn destination(help_text: &'static str, display_order: usize) -> Arg {
        Arg::new(arg_names::DESTINATION_ARG)
            .short('d')
            .long(arg_names::DESTINATION_ARG)
            .help(help_text)
            .takes_value(true)
            .required(true)
            .display_order(display_order)
            .value_name(value_names::TAG)
    }
}

mod cmd {
    use super::*;

    pub fn list() -> Command<'static> {
        Command::new(command_names::LIST)
            .about(about_text::LIST)
            .version(crate_version!())
            .args(&[
                arg::proton::flag(help_text::LIST_PROTON_TAG, 1),
                arg::wine::flag(help_text::LIST_WINE_TAG, 1),
                arg::lol::flag(help_text::LIST_WINE_LOL_TAG, 1),
                arg::newest(help_text::LIST_NEWEST, 2),
                arg::file_system(help_text::LIST_FILE_SYSTEM, 2),
            ])
    }

    pub fn add() -> Command<'static> {
        Command::new(command_names::ADD)
            .about(about_text::ADD)
            .version(crate_version!())
            .args(&[
                arg::proton::with_value(help_text::ADD_PROTON_TAG, 0, 1),
                arg::wine::with_value(help_text::ADD_WINE_TAG, 0, 1),
                arg::lol::with_value(help_text::ADD_WINE_LOL_TAG, 0, 1),
                arg::skip_checksum(help_text::ADD_SKIP_CHECKSUM, 2),
                arg::apply(help_text::ADD_APPLY, 2),
            ])
            .group(group::tag_args(true))
    }

    pub fn remove() -> Command<'static> {
        Command::new(command_names::REMOVE)
            .about(about_text::REMOVE)
            .version(crate_version!())
            .alias("rm")
            .args(&[
                arg::proton::with_value(help_text::REMOVE_PROTON_TAG, 1, 1),
                arg::wine::with_value(help_text::REMOVE_WINE_TAG, 1, 1),
                arg::lol::with_value(help_text::REMOVE_WINE_LOL_TAG, 1, 1),
                arg::forget(help_text::REMOVE_FORGET, 2),
            ])
            .group(group::tag_args(true))
    }

    pub fn check() -> Command<'static> {
        Command::new(command_names::CHECK)
            .about(about_text::CHECK)
            .version(crate_version!())
            .alias("ck")
            .args(&[
                arg::proton::flag(help_text::CHECK_PROTON_TAG, 1),
                arg::wine::flag(help_text::CHECK_WINE_TAG, 1),
                arg::lol::flag(help_text::CHECK_WINE_LOL_TAG, 1),
            ])
            .group(group::tag_args(false))
    }

    pub fn migrate() -> Command<'static> {
        Command::new(command_names::MIGRATE)
            .about(about_text::MIGRATE)
            .version(crate_version!())
            .alias("mg")
            .args(&[
                arg::proton::with_value(help_text::MIGRATE_PROTON_TAG, 1, 1),
                arg::wine::with_value(help_text::MIGRATE_WINE_TAG, 1, 1),
                arg::lol::with_value(help_text::MIGRATE_WINE_LOL_TAG, 1, 1),
            ])
            .arg(
                Arg::new(arg_names::SOURCE_ARG)
                    .short('s')
                    .long(arg_names::SOURCE_ARG)
                    .help(help_text::MIGRATE_SOURCE)
                    .required(true)
                    .takes_value(true)
                    .display_order(1)
                    .value_name(value_names::PATH),
            )
            .group(group::tag_args(true))
    }

    pub fn apply() -> Command<'static> {
        Command::new(command_names::APPLY)
            .about(about_text::APPLY)
            .version(crate_version!())
            .args(&[
                arg::proton::with_value(help_text::APPLY_PROTON_TAG, 1, 1),
                arg::wine::with_value(help_text::APPLY_WINE_TAG, 1, 1),
                arg::lol::with_value(help_text::APPLY_WINE_LOL_TAG, 1, 1),
            ])
            .group(group::tag_args(true))
    }

    pub fn user_settings() -> Command<'static> {
        Command::new(command_names::PROTON_USER_SETTINGS)
            .about(about_text::USER_SETTINGS)
            .version(crate_version!())
            .alias("us")
            .subcommand_required(true)
            .subcommand(
                Command::new(command_names::USER_SETTINGS_COPY)
                    .about(about_text::USER_SETTINGS_COPY)
                    .arg(arg::source(help_text::USER_SETTINGS_COPY_SOURCE, 1))
                    .arg(arg::destination(help_text::USER_SETTINGS_COPY_DESTINATION, 1)),
            )
    }

    pub fn clean() -> Command<'static> {
        Command::new(command_names::CLEAN)
            .about(about_text::CLEAN)
            .version(crate_version!())
            .args(&[
                arg::proton::flag(help_text::CLEAN_PROTON_TAG, 1),
                arg::wine::flag(help_text::CLEAN_WINE_TAG, 1),
                arg::lol::flag(help_text::CLEAN_WINE_LOL_TAG, 1),
                arg::before(help_text::CLEAN_BEFORE_TAG, 2),
                arg::start(help_text::CLEAN_START_TAG, 2),
                arg::end(help_text::CLEAN_END_TAG, 2),
                arg::forget(help_text::CLEAN_FORGET, 3),
                arg::dry_run(help_text::CLEAN_DRY_RUN, 3),
            ])
            .groups(&[
                group::tag_args(true),
                group::start_end_range(false, &[arg_names::BEFORE]),
                group::before_and_start_end(true),
            ])
    }
}

pub fn setup_clap() -> Command<'static> {
    command!()
        .subcommand_required(true)
        .subcommand(cmd::list())
        .subcommand(cmd::add())
        .subcommand(cmd::remove())
        .subcommand(cmd::check())
        .subcommand(cmd::migrate())
        .subcommand(cmd::apply())
        .subcommand(cmd::user_settings())
        .subcommand(cmd::clean())
}

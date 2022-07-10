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
    pub const FORGET: &str = "forget";
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
}

pub mod arg_group_names {
    pub const TAG: &str = "tag-args";
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
    // Forget
    pub const FORGET_PROTON_TAG: &str = "Forget a GE Proton version";
    pub const FORGET_WINE_TAG: &str = "Forget a Wine GE version";
    pub const FORGET_WINE_LOL_TAG: &str = "Forget a Wine GE LoL version";
    // Clean
    pub const CLEAN_PROTON_TAG: &str = "Clean a GE Proton version";
    pub const CLEAN_WINE_TAG: &str = "Clean a Wine GE version";
    pub const CLEAN_WINE_LOL_TAG: &str = "Clean a Wine GE LoL version";
}

pub mod value_name {
    pub const TAG: &str = "TAG";
    pub const PATH: &str = "PATH";
}

fn proton_arg(help_text: &'static str, min_value: usize) -> Arg {
    Arg::new(arg_names::PROTON_ARG)
        .short('p')
        .long(arg_names::PROTON_ARG)
        .help(help_text)
        .display_order(1)
        .takes_value(true)
        .value_name(value_name::TAG)
        .min_values(min_value)
        .max_values(1)
        .multiple_values(false)
}

fn wine_arg(help_text: &'static str, min_value: usize) -> Arg {
    Arg::new(arg_names::WINE_ARG)
        .short('w')
        .long(arg_names::WINE_ARG)
        .help(help_text)
        .display_order(1)
        .takes_value(true)
        .value_name(value_name::TAG)
        .min_values(min_value)
        .max_values(1)
        .multiple_values(false)
}

fn lol_arg(help_text: &'static str, min_value: usize) -> Arg {
    Arg::new(arg_names::LOL_ARG)
        .short('l')
        .long(arg_names::LOL_ARG)
        .help(help_text)
        .display_order(1)
        .takes_value(true)
        .value_name(value_name::TAG)
        .min_values(min_value)
        .max_values(1)
        .multiple_values(false)
}

fn newest_arg(help_text: &'static str) -> Arg {
    Arg::new(arg_names::NEWEST_ARG)
        .short('n')
        .long(arg_names::NEWEST_ARG)
        .display_order(2)
        .help(help_text)
}

fn skip_checksum_arg(help_text: &'static str) -> Arg {
    Arg::new(arg_names::SKIP_CHECKSUM_ARG)
        .long(arg_names::SKIP_CHECKSUM_ARG)
        .display_order(2)
        .help(help_text)
}

fn apply_arg(help_text: &'static str) -> Arg {
    Arg::new(arg_names::APPLY_ARG)
        .long(arg_names::APPLY_ARG)
        .display_order(2)
        .help(help_text)
}

fn tag_arg_group(required: bool) -> ArgGroup<'static> {
    ArgGroup::new(arg_group_names::TAG)
        .args(&[arg_names::PROTON_ARG, arg_names::WINE_ARG, arg_names::LOL_ARG])
        .required(required)
}

fn file_system_arg(help_text: &'static str) -> Arg {
    Arg::new(arg_names::FILE_SYSTEM)
        .short('f')
        .long(arg_names::FILE_SYSTEM)
        .display_order(2)
        .help(help_text)
}

fn setup_list_cmd() -> Command<'static> {
    Command::new(command_names::LIST)
        .about(about_text::LIST)
        .version(crate_version!())
        .args(&[
            proton_arg(help_text::LIST_PROTON_TAG, 0).takes_value(false),
            wine_arg(help_text::LIST_WINE_TAG, 0).takes_value(false),
            lol_arg(help_text::LIST_WINE_LOL_TAG, 0).takes_value(false),
            newest_arg(help_text::LIST_NEWEST),
            file_system_arg(help_text::LIST_FILE_SYSTEM),
        ])
}

fn setup_add_cmd() -> Command<'static> {
    Command::new(command_names::ADD)
        .about(about_text::ADD)
        .version(crate_version!())
        .args(&[
            proton_arg(help_text::ADD_PROTON_TAG, 0),
            wine_arg(help_text::ADD_WINE_TAG, 0),
            lol_arg(help_text::ADD_WINE_LOL_TAG, 0),
            skip_checksum_arg(help_text::ADD_SKIP_CHECKSUM),
            apply_arg(help_text::ADD_APPLY),
        ])
        .group(tag_arg_group(true))
}

fn setup_rm_cmd() -> Command<'static> {
    Command::new(command_names::REMOVE)
        .about(about_text::REMOVE)
        .version(crate_version!())
        .alias("rm")
        .args(&[
            proton_arg(help_text::REMOVE_PROTON_TAG, 1),
            wine_arg(help_text::REMOVE_WINE_TAG, 1),
            lol_arg(help_text::REMOVE_WINE_LOL_TAG, 1),
        ])
        .group(tag_arg_group(true))
}

fn setup_check_cmd() -> Command<'static> {
    Command::new(command_names::CHECK)
        .about(about_text::CHECK)
        .version(crate_version!())
        .alias("ck")
        .args(&[
            proton_arg(help_text::CHECK_PROTON_TAG, 0).takes_value(false),
            wine_arg(help_text::CHECK_WINE_TAG, 0).takes_value(false),
            lol_arg(help_text::CHECK_WINE_LOL_TAG, 0).takes_value(false),
        ])
        .group(tag_arg_group(false))
}

fn setup_migrate_cmd() -> Command<'static> {
    Command::new(command_names::MIGRATE)
        .about(about_text::MIGRATE)
        .version(crate_version!())
        .alias("mg")
        .args(&[
            proton_arg(help_text::MIGRATE_PROTON_TAG, 1),
            wine_arg(help_text::MIGRATE_WINE_TAG, 1),
            lol_arg(help_text::MIGRATE_WINE_LOL_TAG, 1),
        ])
        .arg(
            Arg::new(arg_names::SOURCE_ARG)
                .short('s')
                .long(arg_names::SOURCE_ARG)
                .help(help_text::MIGRATE_SOURCE)
                .required(true)
                .takes_value(true)
                .display_order(1)
                .value_name(value_name::PATH),
        )
        .group(tag_arg_group(true))
}

fn setup_apply_cmd() -> Command<'static> {
    Command::new(command_names::APPLY)
        .about(about_text::APPLY)
        .version(crate_version!())
        .args(&[
            proton_arg(help_text::APPLY_PROTON_TAG, 1),
            wine_arg(help_text::APPLY_WINE_TAG, 1),
            lol_arg(help_text::APPLY_WINE_LOL_TAG, 1),
        ])
        .group(tag_arg_group(true))
}

fn setup_user_settings_cmd() -> Command<'static> {
    Command::new(command_names::PROTON_USER_SETTINGS)
        .about(about_text::USER_SETTINGS)
        .version(crate_version!())
        .alias("us")
        .subcommand_required(true)
        .subcommand(
            Command::new(command_names::USER_SETTINGS_COPY)
                .about(about_text::USER_SETTINGS_COPY)
                .arg(
                    Arg::new(arg_names::SOURCE_ARG)
                        .short('s')
                        .long(arg_names::SOURCE_ARG)
                        .help(help_text::USER_SETTINGS_COPY_SOURCE)
                        .takes_value(true)
                        .required(true)
                        .display_order(1)
                        .value_name(value_name::TAG),
                )
                .arg(
                    Arg::new(arg_names::DESTINATION_ARG)
                        .short('d')
                        .long(arg_names::DESTINATION_ARG)
                        .help(help_text::USER_SETTINGS_COPY_DESTINATION)
                        .takes_value(true)
                        .required(true)
                        .display_order(1)
                        .value_name(value_name::TAG),
                ),
        )
}

fn setup_forget_cmd() -> Command<'static> {
    Command::new(command_names::FORGET)
        .about(about_text::FORGET)
        .version(crate_version!())
        .args(&[
            proton_arg(help_text::FORGET_PROTON_TAG, 1),
            wine_arg(help_text::FORGET_WINE_TAG, 1),
            lol_arg(help_text::FORGET_WINE_LOL_TAG, 1),
        ])
        .group(tag_arg_group(true))
}

fn setup_clean_cmd() -> Command<'static> {
    Command::new(command_names::CLEAN)
        .about(about_text::CLEAN)
        .version(crate_version!())
        .args(&[
            proton_arg(help_text::CLEAN_PROTON_TAG, 1),
            wine_arg(help_text::CLEAN_WINE_TAG, 1),
            lol_arg(help_text::CLEAN_WINE_LOL_TAG, 1),
        ])
}

pub fn setup_clap() -> Command<'static> {
    command!()
        .subcommand_required(true)
        .subcommand(setup_list_cmd())
        .subcommand(setup_add_cmd())
        .subcommand(setup_rm_cmd())
        .subcommand(setup_check_cmd())
        .subcommand(setup_migrate_cmd())
        .subcommand(setup_apply_cmd())
        .subcommand(setup_user_settings_cmd())
        .subcommand(setup_forget_cmd())
        .subcommand(setup_clean_cmd())
}

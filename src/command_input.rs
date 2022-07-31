use std::path::PathBuf;

use anyhow::bail;
use clap::ArgMatches;
use ge_man_lib::tag::{Tag, TagKind, WineTagKind};

use crate::clap::{arg_group_names, arg_names, command_names};
use crate::compat_tool_app::ApplicationConfig;
use crate::data::{ManagedVersion, ManagedVersions};
use crate::filesystem;
use crate::path::PathConfiguration;
use crate::version::{Version, Versioned};

fn application_name(kind: &TagKind) -> String {
    match kind {
        TagKind::Proton => String::from("Steam"),
        TagKind::Wine { .. } => String::from("Lutris"),
    }
}

#[derive(Debug)]
pub struct TagArg {
    pub tag: Option<Tag>,
    pub kind: TagKind,
}

impl TagArg {
    pub fn new(tag: Option<Tag>, kind: TagKind) -> Self {
        let tag = tag.map(Into::into);
        TagArg { tag, kind }
    }

    pub fn value(&self) -> Option<&String> {
        self.tag.as_ref().map(Tag::str)
    }

    pub fn version(&self) -> Version {
        let tag = match &self.tag {
            Some(t) => t,
            None => panic!("TagArg tag value is None. Can not construct version."),
        };

        Version::new(tag.clone(), self.kind)
    }
}

impl TryFrom<&ArgMatches> for TagArg {
    type Error = ();

    fn try_from(matches: &ArgMatches) -> Result<Self, Self::Error> {
        let args: Vec<(&str, TagKind)> = vec![
            (arg_names::PROTON_ARG, TagKind::Proton),
            (arg_names::WINE_ARG, TagKind::wine()),
            (arg_names::LOL_ARG, TagKind::lol()),
        ];

        for (arg, kind) in args {
            if matches.is_present(arg) {
                if let Some(tag) = matches.value_of(arg) {
                    return Ok(TagArg::new(Some(Tag::from(tag)), kind));
                } else {
                    return Ok(TagArg::new(None, kind));
                }
            }
        }

        Err(())
    }
}

pub struct ListCommandInput {
    pub tag_kind: TagKind,
    pub newest: bool,
    pub in_use_directory_name: Option<String>,
    pub managed_versions: ManagedVersions,
    pub application_name: String,
    pub file_system: bool,
    pub app_compat_tool_dir: PathBuf,
}

impl ListCommandInput {
    pub fn new(
        tag_kind: TagKind,
        newest: bool,
        in_use_directory_name: Option<String>,
        managed_versions: ManagedVersions,
        application_name: String,
        file_system: bool,
        app_compat_tool_dir: PathBuf,
    ) -> Self {
        Self {
            tag_kind,
            newest,
            in_use_directory_name,
            managed_versions,
            application_name,
            file_system,
            app_compat_tool_dir,
        }
    }

    pub fn create_from(
        arg_matches: &ArgMatches,
        managed_versions: ManagedVersions,
        path_cfg: &impl PathConfiguration,
    ) -> Vec<ListCommandInput> {
        let matches = arg_matches.subcommand_matches(command_names::LIST).unwrap();
        let tag_kind = TagArg::try_from(matches).ok().map(|tag| tag.kind);
        let file_system = matches.is_present(arg_names::FILE_SYSTEM);

        vec![
            TagKind::Proton,
            TagKind::Wine {
                kind: WineTagKind::WineGe,
            },
        ]
        .into_iter()
        .filter(|kind| Some(kind).eq(&tag_kind.as_ref()) || tag_kind.is_none())
        .map(|kind| {
            let newest = matches.is_present(arg_names::NEWEST_ARG);
            let app_config_file = path_cfg.application_config_file(kind);
            let in_use_directory_name = filesystem::in_use_compat_tool_dir_name(&app_config_file, kind).ok();

            let mut managed_versions = managed_versions.clone();
            managed_versions.retain(|version| version.kind().eq(&kind));
            let app_name = application_name(&kind);
            let app_compat_tool_dir = path_cfg.application_compatibility_tools_dir(kind);

            ListCommandInput::new(
                kind,
                newest,
                in_use_directory_name,
                managed_versions,
                app_name,
                file_system,
                app_compat_tool_dir,
            )
        })
        .collect()
    }
}

#[derive(Debug, Ord, PartialOrd, Eq)]
pub enum GivenVersion {
    Explicit { version: Box<dyn Versioned> },
    Latest { kind: TagKind },
}

impl PartialEq for GivenVersion {
    fn eq(&self, other: &Self) -> bool {
        match self {
            GivenVersion::Explicit { version } => {
                if let GivenVersion::Explicit { version: other_version } = other {
                    version.eq(other_version)
                } else {
                    false
                }
            }
            GivenVersion::Latest { kind } => {
                if let GivenVersion::Latest { kind: other_kind } = other {
                    kind.eq(other_kind)
                } else {
                    false
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct AddCommandInput {
    pub version: GivenVersion,
    pub skip_checksum: bool,
    pub managed_versions: ManagedVersions,
}

impl AddCommandInput {
    pub fn new(version: GivenVersion, skip_checksum: bool, managed_versions: ManagedVersions) -> Self {
        Self {
            version,
            skip_checksum,
            managed_versions,
        }
    }

    pub fn create_from(matches: &ArgMatches, managed_versions: ManagedVersions) -> Self {
        let matches = matches.subcommand_matches(command_names::ADD).unwrap();
        let tag_arg = TagArg::try_from(matches).expect("Could not create tag information from provided argument");
        let skip_checksum = matches.is_present(arg_names::SKIP_CHECKSUM_ARG);

        let version = match tag_arg.tag {
            Some(tag) => GivenVersion::Explicit {
                version: Box::new(Version::new(tag, tag_arg.kind)),
            },
            None => GivenVersion::Latest { kind: tag_arg.kind },
        };
        AddCommandInput::new(version, skip_checksum, managed_versions)
    }

    pub fn apply_present(matches: &ArgMatches) -> bool {
        let matches = matches.subcommand_matches(command_names::ADD).unwrap();
        matches.is_present(arg_names::APPLY_ARG)
    }
}

pub struct RemoveCommandInput {
    pub managed_versions: ManagedVersions,
    pub version_to_remove: ManagedVersion,
    pub app_config_path: PathBuf,
    pub forget: bool,
}

impl RemoveCommandInput {
    pub fn new(
        managed_versions: ManagedVersions,
        version_to_remove: ManagedVersion,
        app_config_path: PathBuf,
        forget: bool,
    ) -> Self {
        Self {
            managed_versions,
            version_to_remove,
            app_config_path,
            forget,
        }
    }

    pub fn create_from(
        matches: &ArgMatches,
        managed_versions: ManagedVersions,
        app_config_path: PathBuf,
    ) -> anyhow::Result<Self> {
        let matches = matches.subcommand_matches(command_names::REMOVE).unwrap();
        let tag_arg = TagArg::try_from(matches).expect("Could not create tag information from provided argument");
        if tag_arg.tag.is_none() {
            bail!("No version provided! - A version is required for removal");
        }
        let forget = matches.is_present(arg_names::FORGET);

        let version = match managed_versions.find_version(&tag_arg.version()) {
            Some(v) => v,
            None => bail!("Given version is not managed"),
        };

        Ok(RemoveCommandInput::new(
            managed_versions,
            version,
            app_config_path,
            forget,
        ))
    }

    pub fn tag_kind_from_matches(matches: &ArgMatches) -> TagKind {
        let matches = matches.subcommand_matches(command_names::REMOVE).unwrap();
        TagArg::try_from(matches).unwrap().kind
    }
}

pub struct CheckCommandInput {
    pub kind: Option<TagKind>,
}

impl CheckCommandInput {
    pub fn new(kind: Option<TagKind>) -> Self {
        CheckCommandInput { kind }
    }
}

impl From<ArgMatches> for CheckCommandInput {
    fn from(matches: ArgMatches) -> Self {
        let matches = matches.subcommand_matches(command_names::CHECK).unwrap();
        if matches.is_present(arg_group_names::TAG) {
            let tag_arg = TagArg::try_from(matches).expect("Could not create tag information from provided argument");
            let kind = tag_arg.kind;
            CheckCommandInput::new(Some(kind))
        } else {
            CheckCommandInput::new(None)
        }
    }
}

impl Default for CheckCommandInput {
    fn default() -> Self {
        CheckCommandInput::new(None)
    }
}

pub struct MigrationCommandInput {
    pub source_path: PathBuf,
    pub version: Version,
    pub managed_versions: ManagedVersions,
}

impl MigrationCommandInput {
    pub fn new(source_path: PathBuf, version: Version, managed_versions: ManagedVersions) -> Self {
        Self {
            source_path,
            version,
            managed_versions,
        }
    }

    pub fn create_from(matches: &ArgMatches, managed_versions: ManagedVersions) -> Self {
        let matches = matches.subcommand_matches(command_names::MIGRATE).unwrap();
        let tag_arg = TagArg::try_from(matches).expect("Could not create tag information from provided argument");
        let source_path = PathBuf::from(matches.value_of(arg_names::SOURCE_ARG).unwrap());

        MigrationCommandInput::new(source_path, tag_arg.version(), managed_versions)
    }
}

pub struct ApplyCommandInput {
    pub version: GivenVersion,
    pub managed_versions: ManagedVersions,
}

impl ApplyCommandInput {
    pub fn new(version: GivenVersion, managed_versions: ManagedVersions) -> Self {
        Self {
            version,
            managed_versions,
        }
    }

    pub fn create_from(matches: &ArgMatches, managed_versions: ManagedVersions) -> Self {
        let matches = matches.subcommand_matches(command_names::APPLY).unwrap();
        let tag_arg = TagArg::try_from(matches).expect("Could not create tag information from provided argument");

        let version = match tag_arg.tag {
            Some(tag) => GivenVersion::Explicit {
                version: Box::new(Version::new(tag, tag_arg.kind)),
            },
            None => GivenVersion::Latest { kind: tag_arg.kind },
        };
        ApplyCommandInput::new(version, managed_versions)
    }
}

#[derive(Debug)]
pub struct CopyUserSettingsCommandInput {
    pub src_version: ManagedVersion,
    pub dst_version: ManagedVersion,
}

impl CopyUserSettingsCommandInput {
    pub fn new(src_version: ManagedVersion, dst_version: ManagedVersion) -> Self {
        Self {
            src_version,
            dst_version,
        }
    }

    pub fn create_from(matches: &ArgMatches, managed_versions: &ManagedVersions) -> anyhow::Result<Self> {
        let matches = matches.subcommand_matches(command_names::PROTON_USER_SETTINGS).unwrap();
        let matches = matches.subcommand_matches(command_names::USER_SETTINGS_COPY).unwrap();
        // Clap handles missing args
        let src_tag = matches.value_of(arg_names::SOURCE_ARG).unwrap();
        let dst_tag = matches.value_of(arg_names::DESTINATION_ARG).unwrap();

        // The user-settings.py file only exists for Proton versions.
        let src_version = match managed_versions.find_version(&Version::new(src_tag, TagKind::Proton)) {
            Some(v) => v,
            None => bail!("Given source Proton version does not exist"),
        };
        let dst_version = match managed_versions.find_version(&Version::new(dst_tag, TagKind::Proton)) {
            Some(v) => v,
            None => bail!("Given destination Proton version does not exist"),
        };
        Ok(CopyUserSettingsCommandInput::new(src_version, dst_version))
    }
}

mod common_clean_input {
    use super::*;

    pub fn tag_kind(matches: &ArgMatches) -> TagKind {
        TagArg::try_from(matches)
            .expect("Could not create tag information from provided argument")
            .kind
    }

    pub fn start(matches: &ArgMatches, tag_kind: TagKind) -> Option<Version> {
        matches
            .value_of(arg_names::START)
            .map(|tag| Version::new(tag, tag_kind))
    }

    pub fn end(matches: &ArgMatches, tag_kind: TagKind) -> Option<Version> {
        matches.value_of(arg_names::END).map(|tag| Version::new(tag, tag_kind))
    }

    pub fn before(matches: &ArgMatches, tag_kind: TagKind) -> Option<Version> {
        matches
            .value_of(arg_names::BEFORE)
            .map(|tag| Version::new(tag, tag_kind))
    }
}

pub struct CleanCommandInput<S, E>
where
    S: Versioned,
    E: Versioned,
{
    pub remove_before_version: Option<S>,
    pub start_version: Option<S>,
    pub end_version: Option<E>,
    pub managed_versions: ManagedVersions,
    pub app_config: ApplicationConfig,
    pub forget: bool,
}

impl<S, E> CleanCommandInput<S, E>
where
    S: Versioned,
    E: Versioned,
{
    pub fn new(
        remove_before_version: Option<S>,
        start_version: Option<S>,
        end_version: Option<E>,
        managed_versions: ManagedVersions,
        app_config: ApplicationConfig,
        forget: bool,
    ) -> Self {
        Self {
            remove_before_version,
            start_version,
            end_version,
            managed_versions,
            app_config: app_config,
            forget,
        }
    }
}

impl CleanCommandInput<Version, Version> {
    pub fn create_from(
        matches: &ArgMatches,
        managed_versions: ManagedVersions,
        app_config_path: PathBuf,
    ) -> anyhow::Result<Self> {
        let matches = matches.subcommand_matches(command_names::CLEAN).unwrap();

        let tag_kind = common_clean_input::tag_kind(matches);
        let start = common_clean_input::start(matches, tag_kind);
        let end = common_clean_input::end(matches, tag_kind);
        let before = common_clean_input::before(matches, tag_kind);
        let forget = matches.is_present(arg_names::FORGET);
        let app_config = ApplicationConfig::create_copy(tag_kind, &app_config_path)?;

        Ok(CleanCommandInput::new(
            before,
            start,
            end,
            managed_versions,
            app_config,
            forget,
        ))
    }

    pub fn tag_kind_from_matches(matches: &ArgMatches) -> TagKind {
        let matches = matches.subcommand_matches(command_names::CLEAN).unwrap();
        common_clean_input::tag_kind(matches)
    }
}

pub struct CleanDryRunInput<'a, S, E>
where
    S: Versioned,
    E: Versioned,
{
    pub remove_before_version: Option<S>,
    pub start_version: Option<S>,
    pub end_version: Option<E>,
    pub managed_versions: &'a ManagedVersions,
}

impl<'a, S, E> CleanDryRunInput<'a, S, E>
where
    S: Versioned,
    E: Versioned,
{
    pub fn new(
        remove_before_version: Option<S>,
        start_version: Option<S>,
        end_version: Option<E>,
        managed_versions: &'a ManagedVersions,
    ) -> Self {
        Self {
            remove_before_version,
            start_version,
            end_version,
            managed_versions,
        }
    }
}

impl<'a> CleanDryRunInput<'a, Version, Version> {
    pub fn create_from(matches: &ArgMatches, managed_versions: &'a ManagedVersions) -> Self {
        let matches = matches.subcommand_matches(command_names::CLEAN).unwrap();
        let tag_kind = common_clean_input::tag_kind(matches);

        let before = common_clean_input::before(matches, tag_kind);
        let start = common_clean_input::start(matches, tag_kind);
        let end = common_clean_input::end(matches, tag_kind);

        CleanDryRunInput::new(before, start, end, managed_versions)
    }

    pub fn is_dry_run(matches: &ArgMatches) -> bool {
        matches
            .subcommand_matches(command_names::CLEAN)
            .unwrap()
            .is_present(arg_names::DRY_RUN)
    }
}

#[cfg(test)]
mod tests {
    use clap::ErrorKind;
    use test_case::test_case;

    use crate::clap::setup_clap;
    use crate::data::ManagedVersion;
    use crate::path::MockPathConfiguration;

    use super::*;

    fn kind_str_to_enum(kind: &str) -> TagKind {
        match kind {
            "-p" => TagKind::Proton,
            "-w" => TagKind::wine(),
            "-l" => TagKind::lol(),
            _ => panic!("Test setup failed: Unexpected kind"),
        }
    }

    fn add_test_template(args: Vec<&str>, expected: AddCommandInput) {
        let matches = setup_clap().try_get_matches_from(args).unwrap();
        let input = AddCommandInput::create_from(&matches, ManagedVersions::default());

        assert_eq!(input.version, expected.version);
        assert_eq!(input.skip_checksum, expected.skip_checksum);
        assert_eq!(input.managed_versions, expected.managed_versions);
    }

    fn remove_test_template(command: Vec<&str>, expected: RemoveCommandInput) {
        let matches = setup_clap().try_get_matches_from(command).unwrap();
        let input =
            RemoveCommandInput::create_from(&matches, expected.managed_versions.clone(), PathBuf::from("/tmp/test"))
                .unwrap();

        assert_eq!(input.version_to_remove, expected.version_to_remove);
        assert_eq!(input.app_config_path, expected.app_config_path);
        assert_eq!(input.managed_versions, expected.managed_versions);
    }

    fn check_test_template(args: Vec<&str>, expected: CheckCommandInput) {
        let matches = setup_clap().try_get_matches_from(args).unwrap();
        let args = CheckCommandInput::from(matches);

        assert_eq!(args.kind, expected.kind);
    }

    fn migration_test_template(command: Vec<&str>, expected: MigrationCommandInput) {
        let matches = setup_clap().try_get_matches_from(command).unwrap();
        let input = MigrationCommandInput::create_from(&matches, ManagedVersions::default());

        assert_eq!(input.source_path, expected.source_path);
        assert_eq!(input.managed_versions, expected.managed_versions);
        assert_eq!(input.version, expected.version);
    }

    fn apply_test_template(args: Vec<&str>, expected: ApplyCommandInput) {
        let matches = setup_clap().try_get_matches_from(args).unwrap();
        let input = ApplyCommandInput::create_from(&matches, ManagedVersions::default());

        assert_eq!(input.version, expected.version);
        assert_eq!(input.managed_versions, expected.managed_versions);
    }

    #[test_case("-p", TagKind::Proton; "Add specific Proton GE version")]
    #[test_case("-w", TagKind::wine(); "Add specific Wine GE version")]
    #[test_case("-l", TagKind::lol(); "Add specific Wine GE LoL version")]
    fn add_specific_proton_tag(kind_arg: &str, kind: TagKind) {
        let tag = "6.20-GE-1";
        let args = vec!["geman", "add", kind_arg, "6.20-GE-1"];
        let expected = AddCommandInput::new(
            GivenVersion::Explicit {
                version: Box::new(Version::new(tag, kind)),
            },
            false,
            ManagedVersions::default(),
        );
        add_test_template(args, expected);
    }

    #[test_case("-p", TagKind::Proton; "Add latest Proton GE version")]
    #[test_case("-w", TagKind::wine(); "Add latest Wine GE version")]
    #[test_case("-l", TagKind::lol(); "Add latest Wine GE LoL version")]
    fn add_latest_tag(kind_arg: &str, kind: TagKind) {
        let args = vec!["geman", "add", kind_arg];
        let expected = AddCommandInput::new(GivenVersion::Latest { kind: kind }, false, ManagedVersions::default());
        add_test_template(args, expected);
    }

    #[test]
    fn add_only_one_tag_arg_allowed() {
        let args = vec!["geman", "add", "-p", "6.20-GE-1", "-w", "6.20-GE-1"];
        let result = setup_clap().try_get_matches_from(args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::ArgumentConflict);
    }

    #[test]
    fn add_with_checksum_skip() {
        let args = vec!["geman", "add", "-p", "--skip-checksum"];
        let expected = AddCommandInput::new(
            GivenVersion::Latest { kind: TagKind::Proton },
            true,
            ManagedVersions::default(),
        );
        add_test_template(args, expected);
    }

    #[test]
    fn add_with_apply() {
        let args = vec!["geman", "add", "-p", "--apply"];
        let expected = AddCommandInput::new(
            GivenVersion::Latest { kind: TagKind::Proton },
            false,
            ManagedVersions::default(),
        );
        add_test_template(args, expected);
    }

    #[test]
    fn add_should_require_one_tag_arg() {
        let args = vec!["geman", "add"];
        let result = setup_clap().try_get_matches_from(args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
    }

    #[test_case("-p", TagKind::Proton; "Remove Proton GE version")]
    #[test_case("-w", TagKind::wine(); "Remove Wine GE version")]
    #[test_case("-l", TagKind::lol(); "Remove Wine GE LoL version")]
    fn remove_specific_tag(kind_arg: &str, kind: TagKind) {
        let command = vec!["geman", "rm", kind_arg, "6.20-GE-1"];
        let managed_versions =
            ManagedVersions::new(vec![ManagedVersion::new("6.20-GE-1", "6.20-GE-1", kind, "6.20-GE-1")]);
        let expected = RemoveCommandInput::new(
            managed_versions,
            ManagedVersion::new("6.20-GE-1", "6.20-GE-1", kind, "6.20-GE-1"),
            PathBuf::from("/tmp/test"),
            false,
        );
        remove_test_template(command, expected);
    }

    #[test_case("-p", TagKind::Proton; "Remove Proton GE version")]
    #[test_case("-w", TagKind::wine(); "Remove Wine GE version")]
    #[test_case("-l", TagKind::lol(); "Remove Wine GE LoL version")]
    fn remove_with_forget_flag(kind_arg: &str, kind: TagKind) {
        let command = vec!["geman", "rm", kind_arg, "6.20-GE-1", "-f"];
        let managed_versions =
            ManagedVersions::new(vec![ManagedVersion::new("6.20-GE-1", "6.20-GE-1", kind, "6.20-GE-1")]);
        let expected = RemoveCommandInput::new(
            managed_versions,
            ManagedVersion::new("6.20-GE-1", "6.20-GE-1", kind, "6.20-GE-1"),
            PathBuf::from("/tmp/test"),
            true,
        );
        remove_test_template(command, expected);
    }

    #[test_case("-p"; "Remove Proton GE version")]
    #[test_case("-w"; "Remove Wine GE version")]
    #[test_case("-l"; "Remove Wine GE LoL version")]
    fn remove_should_require_a_tag_argument_with_a_value(kind: &str) {
        let args = vec!["geman", "rm", kind];
        let result = setup_clap().try_get_matches_from(args);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::EmptyValue);
    }

    #[test_case("-p", TagKind::Proton; "Remove Proton GE version")]
    #[test_case("-w", TagKind::wine(); "Remove Wine GE version")]
    #[test_case("-l", TagKind::lol(); "Remove Wine GE LoL version")]
    fn remove_not_managed_version_should_return_an_error(tag_arg: &str, kind: TagKind) {
        let command = vec!["geman", "rm", tag_arg, "6.20-GE-1"];
        let matches = setup_clap().try_get_matches_from(command).unwrap();

        let managed_versions =
            ManagedVersions::new(vec![ManagedVersion::new("6.21-GE-2", "6.21-GE-2", kind, "6.21-GE-2")]);
        let result = RemoveCommandInput::create_from(&matches, managed_versions.clone(), PathBuf::from("/tmp/test"));
        assert!(result.is_err());
    }

    #[test]
    fn remove_only_one_tag_arg_allowed() {
        let args = vec!["geman", "rm", "-p", "6.20-GE-1", "-w", "6.20-GE-1"];
        let result = setup_clap().try_get_matches_from(args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::ArgumentConflict);
    }

    #[test]
    fn remove_should_require_a_tag_argument() {
        let args = vec!["geman", "rm"];
        let result = setup_clap().try_get_matches_from(args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn check_without_args_should_not_throw_error() {
        let args = vec!["geman", "check"];
        check_test_template(args, CheckCommandInput::default());
    }

    #[test_case("-p"; "Check for Proton GE")]
    #[test_case("-w"; "Check for Wine GE")]
    #[test_case("-l"; "Check for Wine GE LoL")]
    fn check_with_tag_kind(kind: &str) {
        let args = vec!["geman", "check", kind];
        let expected = CheckCommandInput::new(Some(kind_str_to_enum(kind)));
        check_test_template(args, expected);
    }

    #[test]
    fn check_only_one_tag_arg_allowed() {
        let args = vec!["geman", "check", "-p", "-w"];
        let result = setup_clap().try_get_matches_from(args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::ArgumentConflict);
    }

    #[test]
    fn migrate_tag_arg_is_required() {
        let args = vec!["geman", "migrate", "-s", "/tmp"];
        let result = setup_clap().try_get_matches_from(args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
    }

    #[test_case("-p"; "Migration for Proton GE")]
    #[test_case("-w"; "Migration for Wine GE")]
    #[test_case("-l"; "Migration for Wine GE LoL")]
    fn migrate_source_path_is_required(kind: &str) {
        let args = vec!["geman", "migrate", kind, "6.20-GE-1"];
        let result = setup_clap().try_get_matches_from(args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
    }

    #[test_case("-p", TagKind::Proton; "Migrate for GE Proton")]
    #[test_case("-w", TagKind::wine(); "Migrate for Wine GE")]
    #[test_case("-l", TagKind::lol(); "Migrate for Wine GE (LoL)")]
    fn migrate_with_all_required_args(kind_arg: &str, kind: TagKind) {
        let command = vec!["geman", "migrate", kind_arg, "6.20-GE-1", "-s", "/tmp/test"];
        let expected = MigrationCommandInput::new(
            PathBuf::from("/tmp/test"),
            Version::new("6.20-GE-1", kind),
            ManagedVersions::default(),
        );
        migration_test_template(command, expected);
    }

    #[test]
    fn migrate_only_one_tag_arg_allowed() {
        let args = vec!["geman", "migrate", "-s", "/tmp", "-p", "6.20-GE-1", "-w", "6.20-GE-1"];
        let result = setup_clap().try_get_matches_from(args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::ArgumentConflict);
    }

    #[test_case("-p", TagKind::Proton; "Apply for Proton GE")]
    #[test_case("-w", TagKind::wine(); "Apply for Wine GE")]
    #[test_case("-l", TagKind::lol(); "Apply for Wine GE LoL")]
    fn apply_with_all_required_args(kind_arg: &str, kind: TagKind) {
        let args = vec!["geman", "apply", kind_arg, "6.20-GE-1"];
        let expected = ApplyCommandInput::new(
            GivenVersion::Explicit {
                version: Box::new(Version::new("6.20-GE-1", kind)),
            },
            ManagedVersions::default(),
        );
        apply_test_template(args, expected);
    }

    #[test]
    fn apply_missing_required_tag_arg() {
        let args = vec!["geman", "apply"];
        let result = setup_clap().try_get_matches_from(args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
    }

    #[test_case("-p"; "Apply for Proton GE")]
    #[test_case("-w"; "Apply for Wine GE")]
    #[test_case("-l"; "Apply for Wine GE LoL")]
    fn apply_missing_value_for_tag_arg(kind: &str) {
        let args = vec!["geman", "apply", kind];
        let result = setup_clap().try_get_matches_from(args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::EmptyValue);
    }

    #[test]
    fn copy_user_settings_with_all_required_args() {
        let command = vec!["geman", "user-settings", "copy", "-s", "6.20-GE-1", "-d", "6.21-GE-1"];
        let matches = setup_clap().try_get_matches_from(command).unwrap();
        let managed_versions = ManagedVersions::new(vec![
            ManagedVersion::new("6.20-GE-1", "6.20-GE-1", TagKind::Proton, ""),
            ManagedVersion::new("6.21-GE-1", "6.21-GE-1", TagKind::Proton, ""),
        ]);
        let input = CopyUserSettingsCommandInput::create_from(&matches, &managed_versions).unwrap();

        let src_version = ManagedVersion::new("6.20-GE-1", "6.20-GE-1", TagKind::Proton, "");
        let dst_version = ManagedVersion::new("6.21-GE-1", "6.21-GE-1", TagKind::Proton, "");
        let expected = CopyUserSettingsCommandInput::new(src_version, dst_version);

        assert_eq!(input.src_version, expected.src_version);
        assert_eq!(input.dst_version, expected.dst_version);
    }

    #[test]
    fn copy_user_settings_where_stored_source_tag_does_not_exist() {
        let command = vec!["geman", "user-settings", "copy", "-s", "6.20-GE-1", "-d", "6.21-GE-1"];
        let matches = setup_clap().try_get_matches_from(command).unwrap();
        let managed_versions =
            ManagedVersions::new(vec![ManagedVersion::new("6.21-GE-1", "6.21-GE-1", TagKind::Proton, "")]);
        let result = CopyUserSettingsCommandInput::create_from(&matches, &managed_versions);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.to_string(), "Given source Proton version does not exist");
    }

    #[test]
    fn copy_user_settings_where_stored_destination_tag_does_not_exist() {
        let command = vec!["geman", "user-settings", "copy", "-s", "6.20-GE-1", "-d", "6.21-GE-1"];
        let matches = setup_clap().try_get_matches_from(command).unwrap();
        let managed_versions =
            ManagedVersions::new(vec![ManagedVersion::new("6.20-GE-1", "6.20-GE-1", TagKind::Proton, "")]);
        let result = CopyUserSettingsCommandInput::create_from(&matches, &managed_versions);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.to_string(), "Given destination Proton version does not exist");
    }

    #[test]
    fn copy_user_settings_should_fail_without_destination() {
        let args = vec!["geman", "user-settings", "copy", "-s", "6.20-GE-1"];
        let result = setup_clap().try_get_matches_from(args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn copy_user_settings_should_fail_without_source() {
        let args = vec!["geman", "user-settings", "copy", "-d", "6.21-GE-1"];
        let result = setup_clap().try_get_matches_from(args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn list_create_lutris_input() {
        let command = vec!["geman", "list", "-w"];
        let matches = setup_clap().try_get_matches_from(command).unwrap();
        let mut path_cfg = MockPathConfiguration::new();

        let managed_versions = ManagedVersions::new(vec![ManagedVersion::new(
            "6.21-GE-1",
            "6.21-GE-1",
            TagKind::wine(),
            "6.21-GE-1",
        )]);

        path_cfg
            .expect_application_config_file()
            .once()
            .returning(|_| PathBuf::from("test_resources/assets/wine.yml"));
        path_cfg
            .expect_application_compatibility_tools_dir()
            .once()
            .returning(|_| PathBuf::from("/tmp/test"));

        let inputs = ListCommandInput::create_from(&matches, managed_versions.clone(), &path_cfg);
        assert_eq!(inputs.len(), 1);

        let input = &inputs[0];
        assert_eq!(input.managed_versions, managed_versions);
        assert_eq!(input.tag_kind, TagKind::wine());
        assert_eq!(input.newest, false);
        assert_eq!(input.application_name, "Lutris");
        assert_eq!(
            input.in_use_directory_name,
            Some(String::from("lutris-ge-6.21-1-x86_64"))
        );
    }

    #[test]
    fn list_create_steam_input() {
        let command = vec!["geman", "list", "-p"];
        let matches = setup_clap().try_get_matches_from(command).unwrap();
        let mut path_cfg = MockPathConfiguration::new();

        let managed_versions = ManagedVersions::new(vec![ManagedVersion::new(
            "6.21-GE-1",
            "6.21-GE-1",
            TagKind::Proton,
            "6.21-GE-1",
        )]);

        path_cfg
            .expect_application_config_file()
            .once()
            .returning(|_| PathBuf::from("test_resources/assets/config.vdf"));
        path_cfg
            .expect_application_compatibility_tools_dir()
            .once()
            .returning(|_| PathBuf::from("/tmp/test"));

        let inputs = ListCommandInput::create_from(&matches, managed_versions.clone(), &path_cfg);
        assert_eq!(inputs.len(), 1);

        let input = &inputs[0];
        assert_eq!(input.managed_versions, managed_versions);
        assert_eq!(input.tag_kind, TagKind::Proton);
        assert_eq!(input.newest, false);
        assert_eq!(input.application_name, "Steam");
        assert_eq!(input.in_use_directory_name, Some(String::from("Proton-6.21-GE-2")));
    }

    #[test]
    fn clean_input_should_resolve_version_to_remove_before() {
        let command = vec!["geman", "clean", "-p", "-b", "6.21-GE-2"];
        let matches = setup_clap().try_get_matches_from(command).unwrap();
        let managed_versions = ManagedVersions::new(vec![
            ManagedVersion::new("6.20-GE-1", "6.20-GE-1", TagKind::Proton, ""),
            ManagedVersion::new("6.21-GE-1", "6.21-GE-1", TagKind::Proton, ""),
            ManagedVersion::new("6.21-GE-2", "6.21-GE-2", TagKind::Proton, ""),
        ]);

        let input = CleanCommandInput::create_from(
            &matches,
            managed_versions.clone(),
            PathBuf::from("test_resources/assets/config.vdf"),
        )
        .unwrap();
        assert_eq!(
            input.remove_before_version,
            Some(Version::new("6.21-GE-2", TagKind::Proton))
        );
        assert_eq!(input.start_version, None);
        assert_eq!(input.end_version, None);
        assert_eq!(input.managed_versions, managed_versions);
        assert_eq!(
            input.app_config,
            ApplicationConfig::new(TagKind::Proton, "Proton-6.21-GE-2".to_string())
        );
        assert_eq!(input.forget, false);
    }

    #[test]
    fn clean_input_should_resolve_start_and_end_version() {
        let command = vec!["geman", "clean", "-p", "-s", "6.20-GE-1", "-e", "6.21-GE-2"];
        let matches = setup_clap().try_get_matches_from(command).unwrap();
        let managed_versions = ManagedVersions::new(vec![
            ManagedVersion::new("6.20-GE-1", "6.20-GE-1", TagKind::Proton, ""),
            ManagedVersion::new("6.21-GE-1", "6.21-GE-1", TagKind::Proton, ""),
            ManagedVersion::new("6.21-GE-2", "6.21-GE-2", TagKind::Proton, ""),
        ]);

        let input = CleanCommandInput::create_from(
            &matches,
            managed_versions.clone(),
            PathBuf::from("test_resources/assets/config.vdf"),
        )
        .unwrap();
        assert_eq!(input.remove_before_version, None);
        assert_eq!(input.start_version, Some(Version::new("6.20-GE-1", TagKind::Proton)));
        assert_eq!(input.end_version, Some(Version::new("6.21-GE-2", TagKind::Proton)));
        assert_eq!(input.managed_versions, managed_versions);
        assert_eq!(
            input.app_config,
            ApplicationConfig::new(TagKind::Proton, "Proton-6.21-GE-2".to_string())
        );
        assert_eq!(input.forget, false);
    }

    #[test]
    fn clean_with_forget_flag() {
        let command = vec!["geman", "clean", "-p", "-s", "6.20-GE-1", "-e", "6.21-GE-2", "-f"];
        let matches = setup_clap().try_get_matches_from(command).unwrap();
        let managed_versions = ManagedVersions::new(vec![
            ManagedVersion::new("6.20-GE-1", "6.20-GE-1", TagKind::Proton, ""),
            ManagedVersion::new("6.21-GE-1", "6.21-GE-1", TagKind::Proton, ""),
            ManagedVersion::new("6.21-GE-2", "6.21-GE-2", TagKind::Proton, ""),
        ]);

        let input = CleanCommandInput::create_from(
            &matches,
            managed_versions.clone(),
            PathBuf::from("test_resources/assets/config.vdf"),
        )
        .unwrap();
        assert_eq!(input.remove_before_version, None);
        assert_eq!(input.start_version, Some(Version::new("6.20-GE-1", TagKind::Proton)));
        assert_eq!(input.end_version, Some(Version::new("6.21-GE-2", TagKind::Proton)));
        assert_eq!(input.managed_versions, managed_versions);
        assert_eq!(
            input.app_config,
            ApplicationConfig::new(TagKind::Proton, "Proton-6.21-GE-2".to_string())
        );
        assert_eq!(input.forget, true);
    }
}

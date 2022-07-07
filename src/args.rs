use std::path::PathBuf;

use anyhow::bail;
use clap::ArgMatches;
use ge_man_lib::tag::{Tag, TagKind, WineTagKind};

use crate::clap::{arg_group_names, arg_names, command_names};
use crate::data::{ManagedVersion, ManagedVersions};
use crate::path::PathConfiguration;
use crate::version::{Version, Versioned};
use crate::{args, filesystem};

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
        self.tag.as_ref().map(Tag::value)
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
}

impl ListCommandInput {
    pub fn new(
        tag_kind: TagKind,
        newest: bool,
        in_use_directory_name: Option<String>,
        managed_versions: ManagedVersions,
        application_name: String,
    ) -> Self {
        Self {
            tag_kind,
            newest,
            in_use_directory_name,
            managed_versions,
            application_name,
        }
    }

    pub fn create_from(
        arg_matches: &ArgMatches,
        managed_versions: ManagedVersions,
        path_cfg: &impl PathConfiguration,
    ) -> Vec<ListCommandInput> {
        let matches = arg_matches.subcommand_matches(command_names::LIST).unwrap();
        let tag_kind = TagArg::try_from(matches).ok().map(|tag| tag.kind);

        vec![
            TagKind::Proton,
            TagKind::Wine {
                kind: WineTagKind::WineGe,
            },
        ]
        .into_iter()
        .filter(|kind| Some(kind).eq(&tag_kind.as_ref()) || tag_kind.is_none())
        .map(|kind| {
            let app_config_file = path_cfg.application_config_file(&kind);

            let newest = matches.is_present(arg_names::NEWEST_ARG);
            let in_use_directory_name = filesystem::in_use_compat_tool_dir_name(&app_config_file, &kind).ok();

            let mut managed_versions = managed_versions.clone();
            managed_versions.vec_mut().retain(|version| version.kind().eq(&kind));

            let app_name = application_name(&kind);

            ListCommandInput::new(kind, newest, in_use_directory_name, managed_versions, app_name)
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
}

impl RemoveCommandInput {
    pub fn new(managed_versions: ManagedVersions, version_to_remove: ManagedVersion, app_config_path: PathBuf) -> Self {
        Self {
            managed_versions,
            version_to_remove,
            app_config_path,
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

        let version = match managed_versions.find_version(&tag_arg.version()) {
            Some(v) => v,
            None => bail!("Given version is not managed"),
        };

        Ok(RemoveCommandInput::new(managed_versions, version, app_config_path))
    }

    pub fn tag_kind_from_matches(matches: &ArgMatches) -> TagKind {
        let matches = matches.subcommand_matches(args::command_names::REMOVE).unwrap();
        TagArg::try_from(matches).unwrap().kind
    }
}

pub struct CheckArgs {
    pub kind: Option<TagKind>,
}

impl CheckArgs {
    pub fn new(kind: Option<TagKind>) -> Self {
        CheckArgs { kind }
    }
}

impl From<ArgMatches> for CheckArgs {
    fn from(matches: ArgMatches) -> Self {
        let matches = matches.subcommand_matches(command_names::CHECK).unwrap();
        if matches.is_present(arg_group_names::TAG) {
            let tag_arg = TagArg::try_from(matches).expect("Could not create tag information from provided argument");
            let kind = tag_arg.kind;
            CheckArgs::new(Some(kind))
        } else {
            CheckArgs::new(None)
        }
    }
}

impl Default for CheckArgs {
    fn default() -> Self {
        CheckArgs::new(None)
    }
}

pub struct MigrationArgs {
    pub tag_arg: TagArg,
    pub source_path: PathBuf,
}

impl MigrationArgs {
    pub fn new<P: Into<PathBuf>>(tag_arg: TagArg, source_path: P) -> Self {
        let source_path = source_path.into();
        MigrationArgs { tag_arg, source_path }
    }
}

impl From<ArgMatches> for MigrationArgs {
    fn from(matches: ArgMatches) -> Self {
        let matches = matches.subcommand_matches(command_names::MIGRATE).unwrap();
        let tag_arg = TagArg::try_from(matches).expect("Could not create tag information from provided argument");
        let source_path = matches.value_of(arg_names::SOURCE_ARG).unwrap();

        MigrationArgs::new(tag_arg, PathBuf::from(source_path))
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

pub struct CopyUserSettingsArgs {
    pub src_tag: Tag,
    pub dst_tag: Tag,
}

impl CopyUserSettingsArgs {
    pub fn new<T: Into<Tag>>(src_tag: T, dst_tag: T) -> Self {
        let src_tag = src_tag.into();
        let dst_tag = dst_tag.into();
        CopyUserSettingsArgs { src_tag, dst_tag }
    }
}

impl Default for CopyUserSettingsArgs {
    fn default() -> Self {
        CopyUserSettingsArgs::new("", "")
    }
}

impl From<ArgMatches> for CopyUserSettingsArgs {
    fn from(matches: ArgMatches) -> Self {
        let matches = matches.subcommand_matches(command_names::PROTON_USER_SETTINGS).unwrap();
        let matches = matches.subcommand_matches(command_names::USER_SETTINGS_COPY).unwrap();
        let src_tag = matches.value_of(arg_names::SOURCE_ARG).unwrap();
        let dst_tag = matches.value_of(arg_names::DESTINATION_ARG).unwrap();

        CopyUserSettingsArgs::new(src_tag, dst_tag)
    }
}

pub struct ForgetArgs {
    pub tag_arg: TagArg,
}

impl ForgetArgs {
    pub fn new(tag_arg: TagArg) -> Self {
        ForgetArgs { tag_arg }
    }
}

impl From<ArgMatches> for ForgetArgs {
    fn from(matches: ArgMatches) -> Self {
        let matches = matches.subcommand_matches(command_names::FORGET).unwrap();
        let tag_arg = TagArg::try_from(matches).expect("Could not create tag information from provided argument");

        ForgetArgs::new(tag_arg)
    }
}

#[cfg(test)]
mod tests {
    use clap::ErrorKind;
    use test_case::test_case;

    use crate::clap::setup_clap;
    use crate::data::ManagedVersion;
    use crate::path::{MockPathConfiguration, PathConfig};

    use super::*;

    fn assert_tag_arg(tag_arg: TagArg, expected: TagArg) {
        assert_eq!(tag_arg.tag, expected.tag);
        assert_eq!(tag_arg.kind, expected.kind);
    }

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

    fn check_test_template(args: Vec<&str>, expected: CheckArgs) {
        let matches = setup_clap().try_get_matches_from(args).unwrap();
        let args = CheckArgs::from(matches);

        assert_eq!(args.kind, expected.kind);
    }

    fn migration_test_template(args: Vec<&str>, expected: MigrationArgs) {
        let matches = setup_clap().try_get_matches_from(args).unwrap();
        let args = MigrationArgs::from(matches);

        assert_tag_arg(args.tag_arg, expected.tag_arg);
        assert_eq!(args.source_path, expected.source_path);
    }

    fn apply_test_template(args: Vec<&str>, expected: ApplyCommandInput) {
        let matches = setup_clap().try_get_matches_from(args).unwrap();
        let input = ApplyCommandInput::create_from(&matches, ManagedVersions::default());

        assert_eq!(input.version, expected.version);
        assert_eq!(input.managed_versions, expected.managed_versions);
    }

    fn copy_user_settings_test_template(args: Vec<&str>, expected: CopyUserSettingsArgs) {
        let matches = setup_clap().try_get_matches_from(args).unwrap();
        let args = CopyUserSettingsArgs::from(matches);

        assert_eq!(args.src_tag, expected.src_tag);
        assert_eq!(args.dst_tag, expected.dst_tag);
    }

    fn forget_test_template(args: Vec<&str>, expected: ForgetArgs) {
        let matches = setup_clap().try_get_matches_from(args).unwrap();
        let args = ForgetArgs::from(matches);

        assert_tag_arg(args.tag_arg, expected.tag_arg);
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
    fn remove_specific_tag(tag_arg: &str, kind: TagKind) {
        let command = vec!["geman", "rm", tag_arg, "6.20-GE-1"];
        let managed_versions = ManagedVersions::new(vec![ManagedVersion::new("6.20-GE-1", kind, "6.20-GE-1")]);
        let expected = RemoveCommandInput::new(
            managed_versions,
            ManagedVersion::new("6.20-GE-1", kind, "6.20-GE-1"),
            PathBuf::from("/tmp/test"),
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

        let managed_versions = ManagedVersions::new(vec![ManagedVersion::new("6.21-GE-2", kind, "6.21-GE-2")]);
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
        check_test_template(args, CheckArgs::default());
    }

    #[test_case("-p"; "Check for Proton GE")]
    #[test_case("-w"; "Check for Wine GE")]
    #[test_case("-l"; "Check for Wine GE LoL")]
    fn check_with_tag_kind(kind: &str) {
        let args = vec!["geman", "check", kind];
        let expected = CheckArgs::new(Some(kind_str_to_enum(kind)));
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

    #[test]
    fn migrate_with_all_required_args() {
        let args = vec!["geman", "migrate", "-p", "6.20-GE-1", "-s", "/tmp"];
        let expected = MigrationArgs::new(
            TagArg::new(Some(Tag::from("6.20-GE-1")), TagKind::Proton),
            PathBuf::from("/tmp"),
        );
        migration_test_template(args, expected);
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
        let args = vec!["geman", "user-settings", "copy", "-s", "6.20-GE-1", "-d", "6.21-GE-1"];
        let mut expected = CopyUserSettingsArgs::default();
        expected.src_tag = Tag::from("6.20-GE-1");
        expected.dst_tag = Tag::from("6.21-GE-1");
        copy_user_settings_test_template(args, expected);
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

    #[test_case("-p"; "Forget a Proton GE version")]
    #[test_case("-w"; "Forget a Wine GE version")]
    #[test_case("-l"; "Forget a Wine GE LoL version")]
    fn forget_without_tag(kind: &str) {
        let args = vec!["geman", "forget", kind];
        let result = setup_clap().try_get_matches_from(args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::EmptyValue);
    }

    #[test]
    fn forget_without_kind() {
        let args = vec!["geman", "forget"];
        let result = setup_clap().try_get_matches_from(args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn forget_with_all_required_args() {
        let args = vec!["geman", "forget", "-p", "6.20-GE-1"];
        let expected = ForgetArgs::new(TagArg::new(Some(Tag::from("6.20-GE-1")), TagKind::Proton));
        forget_test_template(args, expected);
    }

    #[test]
    fn list_create_lutris_input() {
        let args = vec!["geman", "list", "-w"];
        let matches = setup_clap().try_get_matches_from(args).unwrap();
        let mut path_cfg = MockPathConfiguration::new();

        let managed_versions =
            ManagedVersions::new(vec![ManagedVersion::new("6.21-GE-1", TagKind::wine(), "6.21-GE-1")]);

        path_cfg
            .expect_application_config_file()
            .once()
            .returning(|_| PathBuf::from("test_resources/assets/wine.yml"));

        let inputs = ListCommandInput::create_from(&matches, managed_versions.clone(), &path_cfg);
        assert_eq!(inputs.len(), 1);

        let input = &inputs[0];
        assert_eq!(input.managed_versions, managed_versions);
        assert_eq!(
            input.tag_kind,
            TagKind::Wine {
                kind: WineTagKind::WineGe
            }
        );
        assert_eq!(input.newest, false);
        assert_eq!(input.application_name, "Lutris");
        assert_eq!(
            input.in_use_directory_name,
            Some(String::from("lutris-ge-6.21-1-x86_64"))
        );
    }

    #[test]
    fn list_create_steam_input() {}
}

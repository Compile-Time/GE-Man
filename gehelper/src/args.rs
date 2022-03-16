use std::path::PathBuf;

use crate::clap::{arg_group_names, arg_names, commands};
use clap::ArgMatches;
use gehelper_lib::tag::{Tag, TagKind};

use crate::version::Version;

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

pub struct ListArgs {
    pub kind: Option<TagKind>,
    pub newest: bool,
}

impl ListArgs {
    pub fn new(kind: Option<TagKind>, newest: bool) -> Self {
        ListArgs { kind, newest }
    }
}

impl From<ArgMatches> for ListArgs {
    fn from(matches: ArgMatches) -> Self {
        let matches = matches.subcommand_matches(commands::LIST).unwrap();
        let newest = matches.is_present(arg_names::NEWEST_ARG);
        let kind = TagArg::try_from(matches).ok().map(|tag| tag.kind);

        ListArgs::new(kind, newest)
    }
}

#[derive(Debug)]
pub struct AddArgs {
    pub tag_arg: TagArg,
    pub skip_checksum: bool,
    pub update_config: bool,
}

impl AddArgs {
    pub fn new(tag: TagArg, skip_checksum: bool, update_config: bool) -> Self {
        AddArgs {
            tag_arg: tag,
            skip_checksum,
            update_config,
        }
    }
}

impl From<ArgMatches> for AddArgs {
    fn from(matches: ArgMatches) -> Self {
        let matches = matches.subcommand_matches(commands::ADD).unwrap();
        let tag = TagArg::try_from(matches).expect("Could not create tag information from provided argument");
        let skip_checksum = matches.is_present(arg_names::SKIP_CHECKSUM_ARG);
        let update_config = matches.is_present(arg_names::UPDATE_CONFIG_ARG);

        AddArgs::new(tag, skip_checksum, update_config)
    }
}

pub struct RemoveArgs {
    pub tag_arg: TagArg,
}

impl RemoveArgs {
    pub fn new(tag_arg: TagArg) -> Self {
        RemoveArgs { tag_arg }
    }
}

impl From<ArgMatches> for RemoveArgs {
    fn from(matches: ArgMatches) -> Self {
        let matches = matches.subcommand_matches(commands::REMOVE).unwrap();
        let tag_arg = TagArg::try_from(matches).expect("Could not create tag information from provided argument");
        if tag_arg.tag.is_none() {
            panic!("No version provided!")
        }

        RemoveArgs::new(tag_arg)
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
        let matches = matches.subcommand_matches(commands::CHECK).unwrap();
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
        let matches = matches.subcommand_matches(commands::MIGRATE).unwrap();
        let tag_arg = TagArg::try_from(matches).expect("Could not create tag information from provided argument");
        let source_path = matches.value_of(arg_names::SOURCE_ARG).unwrap();

        MigrationArgs::new(tag_arg, PathBuf::from(source_path))
    }
}

pub struct ApplyArgs {
    pub tag_arg: TagArg,
}

impl ApplyArgs {
    pub fn new(tag_arg: TagArg) -> Self {
        ApplyArgs { tag_arg }
    }
}

impl From<ArgMatches> for ApplyArgs {
    fn from(matches: ArgMatches) -> Self {
        let matches = matches.subcommand_matches(commands::APPLY).unwrap();
        let tag_arg = TagArg::try_from(matches).expect("Could not create tag information from provided argument");
        ApplyArgs::new(tag_arg)
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
        let matches = matches.subcommand_matches(commands::PROTON_USER_SETTINGS).unwrap();
        let matches = matches.subcommand_matches(commands::USER_SETTINGS_COPY).unwrap();
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
        let matches = matches.subcommand_matches(commands::FORGET).unwrap();
        let tag_arg = TagArg::try_from(matches).expect("Could not create tag information from provided argument");

        ForgetArgs::new(tag_arg)
    }
}

#[cfg(test)]
mod tests {
    use crate::clap::setup_clap;
    use clap::ErrorKind;
    use test_case::test_case;

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

    fn add_test_template(args: Vec<&str>, expected: AddArgs) {
        let matches = setup_clap().try_get_matches_from(args).unwrap();
        let args = AddArgs::from(matches);

        assert_tag_arg(args.tag_arg, expected.tag_arg);
        assert_eq!(args.skip_checksum, expected.skip_checksum);
        assert_eq!(args.update_config, expected.update_config);
    }

    fn remove_test_template(args: Vec<&str>, expected: RemoveArgs) {
        let matches = setup_clap().try_get_matches_from(args).unwrap();
        let args = RemoveArgs::from(matches);

        assert_tag_arg(args.tag_arg, expected.tag_arg);
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

    fn apply_test_template(args: Vec<&str>, expected: ApplyArgs) {
        let matches = setup_clap().try_get_matches_from(args).unwrap();
        let args = ApplyArgs::from(matches);

        assert_tag_arg(args.tag_arg, expected.tag_arg);
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

    fn list_test_template(args: Vec<&str>, expected: ListArgs) {
        let matches = setup_clap().try_get_matches_from(args).unwrap();
        let args = ListArgs::from(matches);

        assert_eq!(args.kind, expected.kind);
        assert_eq!(args.newest, expected.newest);
    }

    #[test_case("-p"; "Add specific Proton GE version")]
    #[test_case("-w"; "Add specific Wine GE version")]
    #[test_case("-l"; "Add specific Wine GE LoL version")]
    fn add_specific_proton_tag(kind: &str) {
        let args = vec!["gehelper", "add", kind, "6.20-GE-1"];
        let expected = AddArgs::new(
            TagArg::new(Some(Tag::from("6.20-GE-1")), kind_str_to_enum(kind)),
            false,
            false,
        );
        add_test_template(args, expected);
    }

    #[test_case("-p"; "Add latest Proton GE version")]
    #[test_case("-w"; "Add latest Wine GE version")]
    #[test_case("-l"; "Add latest Wine GE LoL version")]
    fn add_latest_tag(kind: &str) {
        let args = vec!["gehelper", "add", kind];
        let expected = AddArgs::new(TagArg::new(None, kind_str_to_enum(kind)), false, false);
        add_test_template(args, expected);
    }

    #[test]
    fn add_only_one_tag_arg_allowed() {
        let args = vec!["gehelper", "add", "-p", "6.20-GE-1", "-w", "6.20-GE-1"];
        let result = setup_clap().try_get_matches_from(args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::ArgumentConflict);
    }

    #[test]
    fn add_with_checksum_skip() {
        let args = vec!["gehelper", "add", "-p", "--skip-checksum"];
        let expected = AddArgs::new(TagArg::new(None, TagKind::Proton), true, false);
        add_test_template(args, expected);
    }

    #[test]
    fn add_with_update_config() {
        let args = vec!["gehelper", "add", "-p", "--update-config"];
        let expected = AddArgs::new(TagArg::new(None, TagKind::Proton), false, true);
        add_test_template(args, expected);
    }

    #[test]
    fn add_should_require_one_tag_arg() {
        let args = vec!["gehelper", "add"];
        let result = setup_clap().try_get_matches_from(args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
    }

    #[test_case("-p"; "Remove Proton GE version")]
    #[test_case("-w"; "Remove Wine GE version")]
    #[test_case("-l"; "Remove Wine GE LoL version")]
    fn remove_specific_tag(kind: &str) {
        let args = vec!["gehelper", "rm", kind, "6.20-GE-1"];
        let expected = RemoveArgs::new(TagArg::new(Some(Tag::from("6.20-GE-1")), kind_str_to_enum(kind)));
        remove_test_template(args, expected);
    }

    #[test_case("-p"; "Remove Proton GE version")]
    #[test_case("-w"; "Remove Wine GE version")]
    #[test_case("-l"; "Remove Wine GE LoL version")]
    fn remove_should_require_a_tag_argument_with_a_value(kind: &str) {
        let args = vec!["gehelper", "rm", kind];
        let result = setup_clap().try_get_matches_from(args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::EmptyValue);
    }

    #[test]
    fn remove_only_one_tag_arg_allowed() {
        let args = vec!["gehelper", "rm", "-p", "6.20-GE-1", "-w", "6.20-GE-1"];
        let result = setup_clap().try_get_matches_from(args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::ArgumentConflict);
    }

    #[test]
    fn remove_should_require_a_tag_argument() {
        let args = vec!["gehelper", "rm"];
        let result = setup_clap().try_get_matches_from(args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn check_without_args_should_not_throw_error() {
        let args = vec!["gehelper", "check"];
        check_test_template(args, CheckArgs::default());
    }

    #[test_case("-p"; "Check for Proton GE")]
    #[test_case("-w"; "Check for Wine GE")]
    #[test_case("-l"; "Check for Wine GE LoL")]
    fn check_with_tag_kind(kind: &str) {
        let args = vec!["gehelper", "check", kind];
        let expected = CheckArgs::new(Some(kind_str_to_enum(kind)));
        check_test_template(args, expected);
    }

    #[test]
    fn check_only_one_tag_arg_allowed() {
        let args = vec!["gehelper", "check", "-p", "-w"];
        let result = setup_clap().try_get_matches_from(args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::ArgumentConflict);
    }

    #[test]
    fn migrate_tag_arg_is_required() {
        let args = vec!["gehelper", "migrate", "-s", "/tmp"];
        let result = setup_clap().try_get_matches_from(args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
    }

    #[test_case("-p"; "Migration for Proton GE")]
    #[test_case("-w"; "Migration for Wine GE")]
    #[test_case("-l"; "Migration for Wine GE LoL")]
    fn migrate_source_path_is_required(kind: &str) {
        let args = vec!["gehelper", "migrate", kind, "6.20-GE-1"];
        let result = setup_clap().try_get_matches_from(args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn migrate_with_all_required_args() {
        let args = vec!["gehelper", "migrate", "-p", "6.20-GE-1", "-s", "/tmp"];
        let expected = MigrationArgs::new(
            TagArg::new(Some(Tag::from("6.20-GE-1")), TagKind::Proton),
            PathBuf::from("/tmp"),
        );
        migration_test_template(args, expected);
    }

    #[test]
    fn migrate_only_one_tag_arg_allowed() {
        let args = vec![
            "gehelper",
            "migrate",
            "-s",
            "/tmp",
            "-p",
            "6.20-GE-1",
            "-w",
            "6.20-GE-1",
        ];
        let result = setup_clap().try_get_matches_from(args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::ArgumentConflict);
    }

    #[test_case("-p"; "Apply for Proton GE")]
    #[test_case("-w"; "Apply for Wine GE")]
    #[test_case("-l"; "Apply for Wine GE LoL")]
    fn apply_with_all_required_args(kind: &str) {
        let args = vec!["gehelper", "apply", kind, "6.20-GE-1"];
        let expected = ApplyArgs::new(TagArg::new(Some(Tag::from("6.20-GE-1")), kind_str_to_enum(kind)));
        apply_test_template(args, expected);
    }

    #[test]
    fn apply_missing_required_tag_arg() {
        let args = vec!["gehelper", "apply"];
        let result = setup_clap().try_get_matches_from(args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
    }

    #[test_case("-p"; "Apply for Proton GE")]
    #[test_case("-w"; "Apply for Wine GE")]
    #[test_case("-l"; "Apply for Wine GE LoL")]
    fn apply_missing_value_for_tag_arg(kind: &str) {
        let args = vec!["gehelper", "apply", kind];
        let result = setup_clap().try_get_matches_from(args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::EmptyValue);
    }

    #[test]
    fn copy_user_settings_with_all_required_args() {
        let args = vec![
            "gehelper",
            "user-settings",
            "copy",
            "-s",
            "6.20-GE-1",
            "-d",
            "6.21-GE-1",
        ];
        let mut expected = CopyUserSettingsArgs::default();
        expected.src_tag = Tag::from("6.20-GE-1");
        expected.dst_tag = Tag::from("6.21-GE-1");
        copy_user_settings_test_template(args, expected);
    }

    #[test]
    fn copy_user_settings_should_fail_without_destination() {
        let args = vec!["gehelper", "user-settings", "copy", "-s", "6.20-GE-1"];
        let result = setup_clap().try_get_matches_from(args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn copy_user_settings_should_fail_without_source() {
        let args = vec!["gehelper", "user-settings", "copy", "-d", "6.21-GE-1"];
        let result = setup_clap().try_get_matches_from(args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
    }

    #[test_case("-p"; "Forget a Proton GE version")]
    #[test_case("-w"; "Forget a Wine GE version")]
    #[test_case("-l"; "Forget a Wine GE LoL version")]
    fn forget_without_tag(kind: &str) {
        let args = vec!["gehelper", "forget", kind];
        let result = setup_clap().try_get_matches_from(args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::EmptyValue);
    }

    #[test]
    fn forget_without_kind() {
        let args = vec!["gehelper", "forget"];
        let result = setup_clap().try_get_matches_from(args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn forget_with_all_required_args() {
        let args = vec!["gehelper", "forget", "-p", "6.20-GE-1"];
        let expected = ForgetArgs::new(TagArg::new(Some(Tag::from("6.20-GE-1")), TagKind::Proton));
        forget_test_template(args, expected);
    }

    #[test_case("-p"; "Forget a Proton GE version")]
    #[test_case("-w"; "Forget a Wine GE version")]
    #[test_case("-l"; "Forget a Wine GE LoL version")]
    fn list_with_kind_filters(kind: &str) {
        let args = vec!["gehelper", "list", kind];
        let expected = ListArgs::new(Some(kind_str_to_enum(kind)), false);
        list_test_template(args, expected);
    }

    #[test]
    fn list_with_no_args() {
        let args = vec!["gehelper", "list"];
        let expected = ListArgs::new(None, false);
        list_test_template(args, expected);
    }

    #[test]
    fn list_with_latest() {
        let args = vec!["gehelper", "list", "-n"];
        let expected = ListArgs::new(None, true);
        list_test_template(args, expected);
    }
}

use std::io;
use std::io::Write;

use anyhow::{bail, Context};
use ge_man_lib::download::GeDownloader;

use ge_man::clap::command_names::{
    ADD, APPLY, CHECK, CLEAN, LIST, MIGRATE, PROTON_USER_SETTINGS, REMOVE, USER_SETTINGS_COPY,
};
use ge_man::command_execution::CommandHandler;
use ge_man::command_input::{
    AddCommandInput, ApplyCommandInput, CheckCommandInput, CleanCommandInput, CleanDryRunInput,
    CopyUserSettingsCommandInput, GivenVersion, ListCommandInput, MigrationCommandInput, RemoveCommandInput,
};
use ge_man::data::ManagedVersions;
use ge_man::filesystem::FsMng;
use ge_man::path::{overrule, PathConfig, PathConfiguration};
use ge_man::{clap, config};

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let matches = clap::setup_clap().get_matches();

    let stderr = io::stderr();
    let mut err_handle = stderr.lock();

    let path_config = PathConfig::default();

    config::GE_MAN_CONFIG
        .lock()
        .unwrap()
        .read_config(&path_config.ge_man_config(None))?;

    setup_directory_structure(&path_config)?;

    let compatibility_tool_downloader = GeDownloader::default();
    let fs_mng = FsMng::new(&path_config);

    let stdout = io::stdout();
    let mut out_handle = stdout.lock();

    let command_handler = CommandHandler::new(&compatibility_tool_downloader, &fs_mng);
    let result = match matches.subcommand_name() {
        Some(LIST) => {
            let managed_versions_path = path_config.managed_versions_config(overrule::xdg_data_home());
            let managed_versions = ManagedVersions::from_file(&managed_versions_path).context(format!(
                "Could not read managed_versions.json from {}",
                managed_versions_path.display()
            ))?;
            let inputs = ListCommandInput::create_from(&matches, managed_versions, &path_config);

            for input in inputs {
                command_handler.list_versions(&mut out_handle, &mut err_handle, input)?;
                writeln!(out_handle).unwrap();
            }
            Ok(())
        }
        Some(ADD) => {
            let managed_versions_path = path_config.managed_versions_config(overrule::xdg_data_home());
            let managed_versions = ManagedVersions::from_file(&managed_versions_path)?;
            let add_input = AddCommandInput::create_from(&matches, managed_versions);

            let new_and_managed_versions = command_handler.add(&mut out_handle, add_input)?;
            new_and_managed_versions
                .managed_versions
                .write_to_file(&managed_versions_path)?;
            writeln!(out_handle, "Successfully added version").unwrap();

            if AddCommandInput::apply_present(&matches) {
                let apply_input = ApplyCommandInput::new(
                    GivenVersion::Explicit {
                        version: Box::new(new_and_managed_versions.new_versions[0].clone()),
                    },
                    new_and_managed_versions.managed_versions,
                );
                command_handler.apply(&mut out_handle, apply_input)?;
            }

            Ok(())
        }
        Some(REMOVE) => {
            let managed_versions_path = path_config.managed_versions_config(overrule::xdg_data_home());
            let managed_versions = ManagedVersions::from_file(&managed_versions_path)?;
            let app_config_path =
                path_config.application_config_file(RemoveCommandInput::tag_kind_from_matches(&matches));
            let input = RemoveCommandInput::create_from(&matches, managed_versions, app_config_path)?;

            let removed_and_managed_versions = command_handler.remove(input)?;
            removed_and_managed_versions
                .managed_versions
                .write_to_file(&managed_versions_path)?;
            writeln!(
                out_handle,
                "Successfully removed version {}.",
                removed_and_managed_versions.removed_versions[0]
            )
            .unwrap();
            Ok(())
        }
        Some(CHECK) => {
            command_handler.check(&mut out_handle, &mut err_handle, CheckCommandInput::from(matches));
            Ok(())
        }
        Some(MIGRATE) => {
            let managed_versions_path = path_config.managed_versions_config(overrule::xdg_data_home());
            let managed_versions = ManagedVersions::from_file(&managed_versions_path)?;

            let new_and_managed_versions =
                command_handler.migrate(MigrationCommandInput::create_from(&matches, managed_versions))?;

            new_and_managed_versions
                .managed_versions
                .write_to_file(&managed_versions_path)?;
            writeln!(
                out_handle,
                "Successfully migrated directory as {}",
                new_and_managed_versions.new_versions[0]
            )
            .unwrap();
            Ok(())
        }
        Some(APPLY) => {
            let managed_versions_path = path_config.managed_versions_config(overrule::xdg_data_home());
            let managed_versions = ManagedVersions::from_file(&managed_versions_path)?;

            let input = ApplyCommandInput::create_from(&matches, managed_versions);
            command_handler.apply(&mut out_handle, input)
        }
        Some(PROTON_USER_SETTINGS) => {
            let sub_cmd_matches = matches.subcommand_matches(PROTON_USER_SETTINGS).unwrap();
            match sub_cmd_matches.subcommand_name() {
                Some(USER_SETTINGS_COPY) => {
                    let managed_versions_path = path_config.managed_versions_config(overrule::xdg_data_home());
                    let managed_versions = ManagedVersions::from_file(&managed_versions_path)?;

                    let input = CopyUserSettingsCommandInput::create_from(&matches, &managed_versions)?;
                    command_handler.copy_user_settings(&mut out_handle, input)
                }
                _ => Ok(()),
            }
        }
        Some(CLEAN) => {
            let managed_versions_path = path_config.managed_versions_config(overrule::xdg_data_home());
            let managed_versions = ManagedVersions::from_file(&managed_versions_path)?;

            if CleanDryRunInput::is_dry_run(&matches) {
                let input = CleanDryRunInput::create_from(&matches, &managed_versions);
                command_handler.clean_dry_run(&mut out_handle, input)?;
            } else {
                let app_config_path =
                    path_config.application_config_file(CleanCommandInput::tag_kind_from_matches(&matches));
                let input = CleanCommandInput::create_from(&matches, managed_versions, app_config_path)?;

                let mut removed_and_managed_versions = command_handler.clean(&mut err_handle, input)?;
                removed_and_managed_versions
                    .managed_versions
                    .write_to_file(&managed_versions_path)?;

                writeln!(out_handle, "Successfully removed the following versions:").unwrap();
                removed_and_managed_versions
                    .removed_versions
                    .sort_unstable_by(|a, b| a.cmp(b).reverse());
                for version in &removed_and_managed_versions.removed_versions {
                    writeln!(out_handle, "* {}", version.tag()).unwrap();
                }
            }

            Ok(())
        }
        None => Ok(()),
        _ => Ok(()),
    };

    out_handle.flush().unwrap();
    err_handle.flush().unwrap();

    result
}

fn setup_directory_structure(path_config: &PathConfig) -> anyhow::Result<()> {
    if let Err(err) = path_config.create_ge_man_dirs(overrule::xdg_config_home(), overrule::xdg_data_home()) {
        bail!("Failed to setup xdg directory structure: {:#}", err);
    }

    if let Err(err) = path_config.create_app_dirs(
        overrule::xdg_config_home(),
        overrule::xdg_data_home(),
        overrule::steam_root(),
    ) {
        bail!(
            "Failed to setup required directory paths for Steam and Lutris: {:#}",
            err
        );
    }

    Ok(())
}

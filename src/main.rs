use std::io;
use std::io::Write;

use anyhow::{bail, Context};
use ge_man_lib::download::GeDownloader;

use ge_man::args::{
    AddCommandInput, ApplyCommandInput, CheckArgs, CopyUserSettingsArgs, ForgetArgs, GivenVersion, ListCommandInput,
    MigrationArgs, RemoveCommandInput,
};
use ge_man::clap::command_names::{
    ADD, APPLY, CHECK, FORGET, LIST, MIGRATE, PROTON_USER_SETTINGS, REMOVE, USER_SETTINGS_COPY,
};
use ge_man::data::ManagedVersions;
use ge_man::filesystem::FsMng;
use ge_man::path::{overrule, PathConfig, PathConfiguration};
use ge_man::ui::TerminalWriter;
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

    let command_executor = TerminalWriter::new(&compatibility_tool_downloader, &fs_mng, &path_config);
    let result = match matches.subcommand_name() {
        Some(LIST) => {
            let managed_versions_path = path_config.managed_versions_config(overrule::xdg_data_home());
            let managed_versions = ManagedVersions::from_file(&managed_versions_path).context(format!(
                "Could not read managed_versions.json from {}",
                managed_versions_path.display()
            ))?;
            let inputs = ListCommandInput::create_from(&matches, managed_versions, &path_config);

            for input in inputs {
                command_executor.list_versions(&mut out_handle, &mut err_handle, input);
            }
            Ok(())
        }
        Some(ADD) => {
            let managed_versions_path = path_config.managed_versions_config(overrule::xdg_data_home());
            let managed_versions = ManagedVersions::from_file(&managed_versions_path)?;
            let add_input = AddCommandInput::create_from(&matches, managed_versions);

            let new_and_managed_versions = command_executor.add(&mut out_handle, add_input)?;
            new_and_managed_versions
                .managed_versions
                .write_to_file(&managed_versions_path)?;
            writeln!(out_handle, "Successfully added version").unwrap();

            if AddCommandInput::apply_present(&matches) {
                let apply_input = ApplyCommandInput::new(
                    GivenVersion::Explicit {
                        version: Box::new(new_and_managed_versions.version),
                    },
                    new_and_managed_versions.managed_versions,
                );
                command_executor.apply(&mut out_handle, apply_input)?;
            }

            Ok(())
        }
        Some(REMOVE) => {
            let managed_versions_path = path_config.managed_versions_config(overrule::xdg_data_home());
            let managed_versions = ManagedVersions::from_file(&managed_versions_path)?;
            let app_config_path =
                path_config.application_config_file(&RemoveCommandInput::tag_kind_from_matches(&matches));
            let input = RemoveCommandInput::create_from(&matches, managed_versions, app_config_path)?;

            let removed_and_managed_versions = command_executor.remove(input)?;
            removed_and_managed_versions
                .managed_versions
                .write_to_file(&managed_versions_path)?;
            writeln!(
                out_handle,
                "Successfully removed version {}.",
                removed_and_managed_versions.version
            )
            .unwrap();
            Ok(())
        }
        Some(CHECK) => {
            command_executor.check(&mut out_handle, &mut err_handle, CheckArgs::from(matches));
            Ok(())
        }
        Some(MIGRATE) => command_executor.migrate(&mut out_handle, MigrationArgs::from(matches)),
        Some(APPLY) => {
            let managed_versions_path = path_config.managed_versions_config(overrule::xdg_data_home());
            let managed_versions = ManagedVersions::from_file(&managed_versions_path)?;

            let input = ApplyCommandInput::create_from(&matches, managed_versions);
            command_executor.apply(&mut out_handle, input)
        }
        Some(PROTON_USER_SETTINGS) => {
            let sub_cmd_matches = matches.subcommand_matches(PROTON_USER_SETTINGS).unwrap();
            match sub_cmd_matches.subcommand_name() {
                Some(USER_SETTINGS_COPY) => {
                    command_executor.copy_user_settings(&mut out_handle, CopyUserSettingsArgs::from(matches))
                }
                _ => Ok(()),
            }
        }
        Some(FORGET) => command_executor.forget(&mut out_handle, ForgetArgs::from(matches)),
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

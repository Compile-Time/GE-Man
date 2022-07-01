use std::io;
use std::io::Write;

use anyhow::bail;
use ge_man_lib::download::GeDownloader;

use ge_man::args::{
    AddArgs, ApplyArgs, CheckArgs, CopyUserSettingsArgs, ForgetArgs, ListArgs, MigrationArgs, RemoveArgs,
};
use ge_man::clap::command_names::{
    ADD, APPLY, CHECK, FORGET, LIST, MIGRATE, PROTON_USER_SETTINGS, REMOVE, USER_SETTINGS_COPY,
};
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

    let output_writer = TerminalWriter::new(&compatibility_tool_downloader, &fs_mng, &path_config);
    let result = match matches.subcommand_name() {
        Some(LIST) => output_writer.list(&mut out_handle, &mut err_handle, ListArgs::from(matches)),
        Some(ADD) => output_writer.add(&mut out_handle, AddArgs::from(matches)),
        Some(REMOVE) => output_writer.remove(&mut out_handle, RemoveArgs::from(matches)),
        Some(CHECK) => {
            output_writer.check(&mut out_handle, &mut err_handle, CheckArgs::from(matches));
            Ok(())
        }
        Some(MIGRATE) => output_writer.migrate(&mut out_handle, MigrationArgs::from(matches)),
        Some(APPLY) => output_writer.apply_to_app_config(&mut out_handle, ApplyArgs::from(matches)),
        Some(PROTON_USER_SETTINGS) => {
            let sub_cmd_matches = matches.subcommand_matches(PROTON_USER_SETTINGS).unwrap();
            match sub_cmd_matches.subcommand_name() {
                Some(USER_SETTINGS_COPY) => {
                    output_writer.copy_user_settings(&mut out_handle, CopyUserSettingsArgs::from(matches))
                }
                _ => Ok(()),
            }
        }
        Some(FORGET) => output_writer.forget(&mut out_handle, ForgetArgs::from(matches)),
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

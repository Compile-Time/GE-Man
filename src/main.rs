use std::io;
use std::io::Write;

use anyhow::bail;
use ge_man_lib::download::GeDownloader;

use ge_man::args::{
    AddArgs, ApplyArgs, CheckArgs, CopyUserSettingsArgs, ForgetArgs, ListArgs, MigrationArgs, RemoveArgs,
};
use ge_man::clap::commands::{
    ADD, APPLY, CHECK, FORGET, LIST, MIGRATE, PROTON_USER_SETTINGS, REMOVE, USER_SETTINGS_COPY,
};
use ge_man::filesystem::FsMng;
use ge_man::path::{AppConfigPaths, PathConfig};
use ge_man::ui::TerminalWriter;
use ge_man::{clap, path};

fn main() -> anyhow::Result<()> {
    let matches = clap::setup_clap().get_matches();

    let stderr = io::stderr();
    let mut err_handle = stderr.lock();

    let path_config = PathConfig::default();
    if let Err(err) = path::create_xdg_directories(&path_config) {
        bail!("Failed to setup xdg directory structure: {:#}", err);
    }

    let compatibility_tool_downloader = GeDownloader::default();
    let fs_mng = FsMng::new(&path_config);

    let stdout = io::stdout();
    let mut out_handle = stdout.lock();

    let output_writer = TerminalWriter::new(&compatibility_tool_downloader, &fs_mng, &path_config);
    let result = match matches.subcommand_name() {
        Some(LIST) => output_writer.list(
            &mut out_handle,
            ListArgs::from(matches),
            AppConfigPaths::from(&path_config),
        ),
        Some(ADD) => output_writer.add(&mut out_handle, AddArgs::from(matches)),
        Some(REMOVE) => output_writer.remove(
            &mut out_handle,
            RemoveArgs::from(matches),
            AppConfigPaths::from(&path_config),
        ),
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

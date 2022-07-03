use ge_man_lib::tag::TagKind;

pub mod apply {
    use super::*;
    use crate::version::Versioned;

    const STEAM_MODIFYING_CONFIG_MSG: &str = "Modifying Steam configuration to use";
    const STEAM_SUCESS_MSG: &str = "Successfully modified Steam config: If Steam is currently running, \
    any external change by GE-Man will not take effect and the new version can not be selected in the Steam settings!

    To select the latest version you have two options:
    \t1. Restart Steam to select the new version in Steam (which then requires a second restart for Steam to register the change).
    \t2. Close Steam and run the apply command for your desired version. On the next start Steam will use the applied version.";

    const LUTRIS_MODIFYING_CONFIG_MSG: &str = "Modifying Lutris configuration to use";
    const LUTRIS_SUCCESS_MSG: &str = "Successfully modified Lutris config: Lutris should be restarted for the new \
    settings to take effect.";

    pub fn modifying_config(version: &dyn Versioned) -> String {
        match version.kind() {
            TagKind::Proton => format!("{} {}", STEAM_MODIFYING_CONFIG_MSG, version.tag()),
            TagKind::Wine { .. } => format!("{} {}", LUTRIS_MODIFYING_CONFIG_MSG, version.tag()),
        }
    }

    pub fn modify_config_success(kind: &TagKind) -> &str {
        match kind {
            TagKind::Proton => STEAM_SUCESS_MSG,
            TagKind::Wine { .. } => LUTRIS_SUCCESS_MSG,
        }
    }
}

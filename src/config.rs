#[allow(unused_imports)]
use bevy::prelude::{info, Local, Plugin, Res, ResMut};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct EmulatorConfig {
    #[serde(default = "default_true")]
    pub collisions: bool,

    #[serde(default = "default_scale")]
    pub scale: f32,

    #[serde(default)]
    pub arrows_force_ctl: bool,

    #[serde(default = "default_true")]
    pub arrows_neg_ctl: bool,

    #[serde(default = "default_true")]
    pub arrows_joystick: bool,

    #[serde(default)]
    pub basic: bool,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct LocalEmulatorConfig {
    #[serde(default)]
    pub collisions: Option<bool>,

    #[serde(default = "default_scale")]
    pub scale: f32,

    #[serde(default)]
    pub arrows_force_ctl: bool,

    #[serde(default = "default_true")]
    pub arrows_neg_ctl: bool,

    #[serde(default = "default_true")]
    pub arrows_joystick: bool,

    #[serde(default)]
    pub basic: Option<bool>,
}

fn default_scale() -> f32 {
    2.0
}
fn default_true() -> bool {
    true
}

impl EmulatorConfig {
    pub fn is_multi(&self) -> bool {
        false
    }
}

pub struct GlobalEmulatorConfig(pub EmulatorConfig);

pub fn merge(global: &EmulatorConfig, local: &LocalEmulatorConfig) -> EmulatorConfig {
    let mut global_obj = serde_json::to_value(global)
        .unwrap()
        .as_object()
        .unwrap()
        .clone();
    let local_val = serde_json::to_value(local).unwrap();
    let local_obj = local_val.as_object().unwrap();
    for (k, v) in local_obj.iter() {
        if !v.is_null() {
            global_obj.insert(k.to_owned(), v.to_owned());
        }
    }
    serde_json::from_value(serde_json::value::Value::Object(global_obj)).unwrap()
}

#[cfg(target_arch = "wasm32")]
fn merge_configs(
    prev_state: &mut PrevState,
    global: &mut EmulatorConfig,
    config: &mut EmulatorConfig,
) -> bool {
    let window = web_sys::window().unwrap();
    let local_storage = window.local_storage().unwrap().unwrap();
    let config_json = if let Ok(Some(config_json)) = local_storage.get_item("config") {
        config_json
    } else {
        "{}".to_string()
    };

    let hash = window.location().hash().unwrap_or("".to_string());
    let hash = if hash.starts_with("#") {
        &hash[1..]
    } else {
        &hash
    };

    if config_json != prev_state.config || hash != prev_state.hash {
        let global_config: EmulatorConfig = serde_json::from_str(&config_json).unwrap();
        let local: LocalEmulatorConfig = serde_urlencoded::from_str(hash).unwrap();
        *config = merge(&global_config, &local);
        *global = global_config;
        prev_state.config = config_json;
        prev_state.hash = hash.to_string();
        true
    } else {
        false
    }
}

#[cfg(target_arch = "wasm32")]
fn read_config(
    mut prev_state: ResMut<PrevState>,
    mut global: ResMut<GlobalEmulatorConfig>,
    mut config: ResMut<EmulatorConfig>,
    mut ui_config: ResMut<crate::resources::UIConfig>,
) {
    if merge_configs(&mut prev_state, &mut global.0, &mut config) {
        info!("changed: {:?}", *config);
        ui_config.basic = config.basic;
    }
}

#[cfg(target_arch = "wasm32")]
fn store_config(config: Res<GlobalEmulatorConfig>, mut local: Local<Option<EmulatorConfig>>) {
    if let Some(l) = &*local {
        if *l != config.0 {
            if let Ok(serialized) = serde_json::to_string_pretty(&config.0) {
                info!("config modified: {}", serialized);
                #[cfg(target_arch = "wasm32")]
                if let Ok(Some(local_storage)) = web_sys::window().unwrap().local_storage() {
                    local_storage
                        .set_item("config", &serialized)
                        .expect("written");
                }
            }
            *local = Some(config.0.clone())
        }
    } else {
        *local = Some(config.0.clone());
    }
}

#[derive(Default)]
pub struct ConfigPlugin;

#[cfg(target_arch = "wasm32")]
#[derive(Default)]
struct PrevState {
    hash: String,
    config: String,
}

impl Plugin for ConfigPlugin {
    #[cfg(target_arch = "wasm32")]
    fn build(&self, app: &mut bevy::prelude::App) {
        use bevy::prelude::CoreStage;

        let mut prev_state = PrevState::default();
        let mut emulator_config: EmulatorConfig = serde_json::from_str("{}").unwrap();
        let mut global_config = GlobalEmulatorConfig(emulator_config.clone());
        merge_configs(&mut prev_state, &mut global_config.0, &mut emulator_config);
        app.insert_resource(prev_state);
        app.insert_resource(emulator_config);
        app.insert_resource(global_config);
        app.add_system_to_stage(CoreStage::PreUpdate, read_config);
        app.add_system_to_stage(CoreStage::PostUpdate, store_config);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn build(&self, _app: &mut bevy::prelude::App) {}
}

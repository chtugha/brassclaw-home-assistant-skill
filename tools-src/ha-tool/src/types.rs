use crate::shell::SshConfig;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum HaAction {
    GetStatus {
        ha_url: String,
    },

    GetStates {
        ha_url: String,
        #[serde(default)]
        domain_filter: Option<String>,
        #[serde(default)]
        max_items: Option<u32>,
    },

    GetState {
        ha_url: String,
        entity_id: String,
    },

    SetState {
        ha_url: String,
        entity_id: String,
        state: String,
        #[serde(default)]
        attributes: Option<serde_json::Value>,
    },

    CallService {
        ha_url: String,
        domain: String,
        service: String,
        #[serde(default)]
        data: Option<serde_json::Value>,
    },

    GetServices {
        ha_url: String,
    },

    FireEvent {
        ha_url: String,
        event_type: String,
        #[serde(default)]
        event_data: Option<serde_json::Value>,
    },

    RenderTemplate {
        ha_url: String,
        template: String,
    },

    GetHistory {
        ha_url: String,
        entity_id: String,
        #[serde(default = "default_hours_back")]
        hours_back: u32,
        #[serde(default)]
        start_time: Option<String>,
    },

    GetLogbook {
        ha_url: String,
        #[serde(default)]
        entity_id: Option<String>,
        #[serde(default = "default_hours_back")]
        hours_back: u32,
    },

    GetCalendarEvents {
        ha_url: String,
        entity_id: String,
        start: String,
        end: String,
    },

    ListAutomations {
        ha_url: String,
    },

    ToggleAutomation {
        ha_url: String,
        entity_id: String,
        #[serde(default = "default_enabled")]
        enabled: bool,
    },

    TriggerAutomation {
        ha_url: String,
        entity_id: String,
    },

    ListScripts {
        ha_url: String,
    },

    RunScript {
        ha_url: String,
        entity_id: String,
        #[serde(default)]
        variables: Option<serde_json::Value>,
    },

    ListScenes {
        ha_url: String,
    },

    ActivateScene {
        ha_url: String,
        entity_id: String,
    },

    MqttPublish {
        ha_url: String,
        topic: String,
        payload: String,
        #[serde(default)]
        qos: Option<u8>,
        #[serde(default)]
        retain: Option<bool>,
    },

    ModbusWrite {
        ha_url: String,
        #[serde(default)]
        hub: Option<String>,
        unit: u16,
        address: u16,
        value: serde_json::Value,
        write_type: String,
    },

    GetConfig {
        ha_url: String,
    },

    GetNotifications {
        ha_url: String,
    },

    DismissNotification {
        ha_url: String,
        notification_id: String,
    },

    CheckConfig {
        ha_url: String,
        #[serde(default)]
        ssh: Option<SshConfig>,
    },

    GetErrorLog {
        ha_url: String,
        #[serde(default)]
        tail_lines: Option<u32>,
        #[serde(default)]
        ssh: Option<SshConfig>,
        #[serde(default)]
        log_path: Option<String>,
    },

    RestartHa {
        ha_url: String,
        #[serde(default)]
        ssh: Option<SshConfig>,
    },

    ShellStatus {
        #[serde(default)]
        gateway_port: Option<u16>,
    },

    ShellExec {
        ssh: SshConfig,
        command: String,
        #[serde(default)]
        timeout_secs: Option<u32>,
    },

    ShellReadFile {
        ssh: SshConfig,
        path: String,
    },

    ShellWriteFile {
        ssh: SshConfig,
        path: String,
        content: String,
    },

    ShellTailFile {
        ssh: SshConfig,
        path: String,
        lines: u32,
    },

    HaCli {
        ssh: SshConfig,
        args: String,
    },

    ReloadCoreConfig {
        ha_url: String,
    },

    ReloadAutomations {
        ha_url: String,
    },

    ReloadScripts {
        ha_url: String,
    },

    ReloadScenes {
        ha_url: String,
    },

    ReloadThemes {
        ha_url: String,
    },

    ReloadConfigEntry {
        ha_url: String,
        entry_id: String,
    },
}

fn default_hours_back() -> u32 {
    24
}

fn default_enabled() -> bool {
    true
}

#[derive(Debug, Serialize)]
pub struct StatesResponse {
    pub entities: Vec<serde_json::Value>,
    pub count: usize,
    pub total: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncated: Option<bool>,
}

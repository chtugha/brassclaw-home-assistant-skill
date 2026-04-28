use crate::shell::SshConfig;
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Serialize};

/// Schemars 1 emits the boolean schema `true` for `serde_json::Value`
/// when used in a non-Option position. Several LLM tool-call parsers
/// (e.g. Ollama's `ToolFunctionParameters`) reject boolean schemas
/// under `properties.<key>` and expect an object schema. This helper
/// emits an explicit object schema that accepts any JSON value while
/// remaining structurally an object.
fn any_json_value_schema(_: &mut SchemaGenerator) -> Schema {
    schemars::json_schema!({
        "description": "Any JSON value: boolean, integer, number, string, array, object, or null.",
        "type": ["boolean", "integer", "number", "string", "array", "object", "null"]
    })
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum StringOrVec {
    Single(String),
    Many(Vec<String>),
}

impl StringOrVec {
    pub fn as_vec(&self) -> Vec<&str> {
        match self {
            StringOrVec::Single(s) => vec![s.as_str()],
            StringOrVec::Many(v) => v.iter().map(String::as_str).collect(),
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum HaAction {
    GetStatus {
        ha_url: String,
    },

    GetStates {
        ha_url: String,
        #[serde(default)]
        domain_filter: Option<StringOrVec>,
        #[serde(default)]
        max_items: Option<u32>,
        #[serde(default)]
        compact: Option<bool>,
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
        #[schemars(schema_with = "any_json_value_schema")]
        attributes: Option<serde_json::Value>,
    },

    DeleteState {
        ha_url: String,
        entity_id: String,
    },

    CallService {
        ha_url: String,
        domain: String,
        service: String,
        #[serde(default)]
        #[schemars(schema_with = "any_json_value_schema")]
        data: Option<serde_json::Value>,
    },

    GetServices {
        ha_url: String,
    },

    FireEvent {
        ha_url: String,
        event_type: String,
        #[serde(default)]
        #[schemars(schema_with = "any_json_value_schema")]
        event_data: Option<serde_json::Value>,
    },

    RenderTemplate {
        ha_url: String,
        template: String,
        #[serde(default)]
        #[schemars(schema_with = "any_json_value_schema")]
        variables: Option<serde_json::Value>,
        #[serde(default)]
        max_chars: Option<u32>,
    },

    GetHistory {
        ha_url: String,
        entity_id: String,
        #[serde(default = "default_hours_back")]
        hours_back: u32,
        #[serde(default)]
        start_time: Option<String>,
        #[serde(default)]
        end_time: Option<String>,
    },

    GetLogbook {
        ha_url: String,
        #[serde(default)]
        entity_id: Option<String>,
        #[serde(default = "default_hours_back")]
        hours_back: u32,
        #[serde(default)]
        start_time: Option<String>,
        #[serde(default)]
        end_time: Option<String>,
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
        #[schemars(schema_with = "any_json_value_schema")]
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
        #[schemars(schema_with = "any_json_value_schema")]
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

    GetConfigEntries {
        ha_url: String,
        #[serde(default)]
        domain: Option<String>,
    },
}

fn default_hours_back() -> u32 {
    24
}

fn default_enabled() -> bool {
    true
}

/// Response for `get_states`. `matched` is the filtered count (after
/// `domain_filter`); `total` is the full unfiltered entity count reported
/// by Home Assistant. `count` is the number actually returned (may be
/// less than `matched` if `max_items`/hard cap truncated the list).
#[derive(Debug, Serialize)]
pub struct StatesResponse {
    pub entities: Vec<serde_json::Value>,
    pub count: usize,
    pub matched: usize,
    pub total: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncated: Option<bool>,
    /// "user" when truncated by caller-supplied `max_items`,
    /// "hard" when truncated by the server-side safety cap.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cap_kind: Option<&'static str>,
}

#[cfg(test)]
mod tests {
    use super::HaAction;

    /// Schemars 1 emits boolean schemas (`true`/`false`) for permissive
    /// types like `serde_json::Value`. Several LLM tool-call parsers —
    /// notably Ollama's `ToolFunctionParameters` — reject any boolean
    /// schema under `properties.<key>` and expect a JSON object schema.
    /// Walk the generated schema and assert every value under a
    /// `properties` map is an object.
    #[test]
    fn schema_contains_no_boolean_property_schemas() {
        let schema = schemars::schema_for!(HaAction);
        let value = serde_json::to_value(&schema).expect("schema serializes");
        let mut offenders = Vec::new();
        check_no_bool_in_properties("$", &value, &mut offenders);
        assert!(
            offenders.is_empty(),
            "boolean schemas under `properties` (Ollama-incompatible): {:#?}",
            offenders
        );
    }

    fn check_no_bool_in_properties(path: &str, v: &serde_json::Value, out: &mut Vec<String>) {
        match v {
            serde_json::Value::Object(map) => {
                if let Some(serde_json::Value::Object(props)) = map.get("properties") {
                    for (k, vv) in props {
                        if vv.is_boolean() {
                            out.push(format!("{}.properties.{}", path, k));
                        } else {
                            check_no_bool_in_properties(
                                &format!("{}.properties.{}", path, k),
                                vv,
                                out,
                            );
                        }
                    }
                }
                for (k, vv) in map {
                    if k == "properties" {
                        continue;
                    }
                    check_no_bool_in_properties(&format!("{}.{}", path, k), vv, out);
                }
            }
            serde_json::Value::Array(arr) => {
                for (i, vv) in arr.iter().enumerate() {
                    check_no_bool_in_properties(&format!("{}[{}]", path, i), vv, out);
                }
            }
            _ => {}
        }
    }
}

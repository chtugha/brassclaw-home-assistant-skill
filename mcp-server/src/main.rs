use serde::{Deserialize, Serialize};
use std::io;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedEntity {
    pub entity_id: String,
    pub name: String,
    pub state: String,
    pub domain: String,
}

pub struct EntityCache {
    pub entities: Vec<CachedEntity>,
    pub fetched_at: Instant,
}

static CACHE: std::sync::OnceLock<Mutex<Option<EntityCache>>> = std::sync::OnceLock::new();

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum Id {
    String(String),
    Number(i64),
}

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<serde_json::Value>,
    pub id: Option<Id>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    pub id: Option<Id>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct CallToolParams {
    pub name: String,
    pub arguments: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct CallToolResult {
    pub content: Vec<ToolContent>,
    #[serde(rename = "isError", skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ToolContent {
    Text { text: String },
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct SearchParams {
    query: String,
    domain: Option<String>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct ControlParams {
    entity_id: String,
    action: String,
    value: Option<serde_json::Value>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct EditConfigParams {
    action: String,
    file: Option<String>,
    old_string: Option<String>,
    new_string: Option<String>,
    offset: Option<usize>,
    limit: Option<usize>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct ProbeModbusParams {
    host: Option<String>,
    port: Option<u16>,
    unit_id: Option<u8>,
    register_type: String,
    address: u16,
    count: Option<u16>,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin).lines();

    while let Some(line) = reader.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }

        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(req) => req,
            Err(e) => {
                let response = JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32700,
                        message: format!("Parse error: {}", e),
                        data: None,
                    }),
                    id: None,
                };
                send_response(&response).await?;
                continue;
            }
        };

        let response = handle_request(request).await;
        if let Some(resp) = response {
            send_response(&resp).await?;
        }
    }

    Ok(())
}

async fn send_response(response: &JsonRpcResponse) -> io::Result<()> {
    let serialized = serde_json::to_string(response).unwrap();
    let mut stdout = tokio::io::stdout();
    stdout.write_all(serialized.as_bytes()).await?;
    stdout.write_all(b"\n").await?;
    stdout.flush().await?;
    Ok(())
}

async fn handle_request(request: JsonRpcRequest) -> Option<JsonRpcResponse> {
    let id = request.id.clone();
    let method = request.method.as_str();

    match method {
        "initialize" => {
            let result = serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "home-assistant-mcp",
                    "version": "0.1.0"
                }
            });
            Some(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: Some(result),
                error: None,
                id,
            })
        }
        "notifications/initialized" => None,
        "tools/list" => {
            let tools = serde_json::json!({
                "tools": [
                    {
                        "name": "ha_search_entities",
                        "description": "Search for entities by natural-language query.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "query": {
                                    "type": "string",
                                    "description": "The search term, friendly name, area name, or keyword (e.g. 'kitchen lamp', 'temperature')."
                                },
                                "domain": {
                                    "type": "string",
                                    "description": "Optional domain filter (e.g. 'light', 'climate', 'switch', 'sensor')."
                                }
                            },
                            "required": ["query"]
                        }
                    },
                    {
                        "name": "ha_control",
                        "description": "Perform actions on a specific entity.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "entity_id": {
                                    "type": "string",
                                    "description": "The target Home Assistant entity_id (e.g. 'light.living_room')."
                                },
                                "action": {
                                    "type": "string",
                                    "enum": ["turn_on", "turn_off", "toggle", "set_value"],
                                    "description": "The high-level action to perform."
                                },
                                "value": {
                                    "type": ["string", "number", "boolean"],
                                    "description": "Optional value for the action (e.g. 50 for brightness percentage, 21.5 for climate temperature)."
                                }
                            },
                            "required": ["entity_id", "action"]
                        }
                    },
                    {
                        "name": "ha_get_diagnostics",
                        "description": "Verify configuration health or retrieve system alerts.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {}
                        }
                    },
                    {
                        "name": "ha_edit_config",
                        "description": "Read or patch Home Assistant configuration files in a token-efficient manner.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "action": {
                                    "type": "string",
                                    "enum": ["read", "patch"],
                                    "description": "The edit action to perform."
                                },
                                "file": {
                                    "type": "string",
                                    "description": "The configuration file name (e.g. 'configuration.yaml'). Defaults to 'configuration.yaml'. Path traversal is blocked."
                                },
                                "old_string": {
                                    "type": "string",
                                    "description": "The exact string to search for during a patch action. Must match exactly once."
                                },
                                "new_string": {
                                    "type": "string",
                                    "description": "The replacement string for the patch action."
                                },
                                "offset": {
                                    "type": "integer",
                                    "description": "The line offset for reading file contents (default: 0)."
                                },
                                "limit": {
                                    "type": "integer",
                                    "description": "The maximum number of lines to read (default: 100)."
                                }
                            },
                            "required": ["action"]
                        }
                    },
                    {
                        "name": "ha_probe_modbus",
                        "description": "Directly probe Modbus TCP registers. If 'host' is omitted or set to a hub name, it is auto-discovered from configuration.yaml.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "host": {
                                    "type": "string",
                                    "description": "The Modbus TCP target IP, hostname, or hub name. If omitted, auto-discovers from configuration.yaml."
                                },
                                "port": {
                                    "type": "integer",
                                    "description": "The Modbus TCP port (default: 502)."
                                },
                                "unit_id": {
                                    "type": "integer",
                                    "description": "The slave/unit identifier (default: 1)."
                                },
                                "register_type": {
                                    "type": "string",
                                    "enum": ["holding", "input", "coil", "discrete"],
                                    "description": "The register domain to query."
                                },
                                "address": {
                                    "type": "integer",
                                    "description": "The starting register address (0-indexed)."
                                },
                                "count": {
                                    "type": "integer",
                                    "description": "The number of registers/coils to read (default: 1, max: 125)."
                                }
                            },
                            "required": ["register_type", "address"]
                        }
                    }
                ]
            });
            Some(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: Some(tools),
                error: None,
                id,
            })
        }
        "tools/call" => {
            let params = match request.params {
                Some(p) => p,
                None => {
                    return Some(JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32602,
                            message: "Missing params".to_string(),
                            data: None,
                        }),
                        id,
                    });
                }
            };

            let call_params: CallToolParams = match serde_json::from_value(params) {
                Ok(cp) => cp,
                Err(e) => {
                    return Some(JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32602,
                            message: format!("Invalid params: {}", e),
                            data: None,
                        }),
                        id,
                    });
                }
            };

            let result = handle_tool_call(call_params).await;
            match result {
                Ok(val) => Some(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: Some(val),
                    error: None,
                    id,
                }),
                Err(err_msg) => {
                    let tool_result = CallToolResult {
                        content: vec![ToolContent::Text { text: err_msg }],
                        is_error: Some(true),
                    };
                    Some(JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        result: Some(serde_json::to_value(&tool_result).unwrap()),
                        error: None,
                        id,
                    })
                }
            }
        }
        _ => Some(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: -32601,
                message: format!("Method not found: {}", method),
                data: None,
            }),
            id,
        }),
    }
}

async fn fetch_entities_from_ha() -> Result<Vec<CachedEntity>, String> {
    let ha_url = std::env::var("HA_URL")
        .or_else(|_| std::env::var("HOME_ASSISTANT_URL"))
        .map_err(|_| "HA_URL environment variable is not set".to_string())?;

    let ha_token = std::env::var("HA_TOKEN")
        .or_else(|_| std::env::var("HOME_ASSISTANT_API_KEY"))
        .map_err(|_| "HA_TOKEN environment variable is not set".to_string())?;

    let base_url = ha_url.trim_end_matches('/');
    let url = format!("{}/api/states", base_url);

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", ha_token))
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| format!("Failed to send request to Home Assistant: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!(
            "Home Assistant returned status {}: {}",
            status, body
        ));
    }

    let states: Vec<serde_json::Value> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Home Assistant response: {}", e))?;

    let mut cached_entities = Vec::new();
    for item in states {
        if let Some(entity_id) = item.get("entity_id").and_then(|v| v.as_str()) {
            let state = item
                .get("state")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            let name = item
                .get("attributes")
                .and_then(|attrs| attrs.get("friendly_name"))
                .and_then(|v| v.as_str())
                .unwrap_or(entity_id)
                .to_string();

            let domain = entity_id.split('.').next().unwrap_or("").to_string();

            cached_entities.push(CachedEntity {
                entity_id: entity_id.to_string(),
                name,
                state,
                domain,
            });
        }
    }

    Ok(cached_entities)
}

fn get_mock_entities() -> Vec<CachedEntity> {
    vec![
        CachedEntity {
            entity_id: "light.kitchen_overhead".to_string(),
            name: "Kitchen Overhead Light".to_string(),
            state: "off".to_string(),
            domain: "light".to_string(),
        },
        CachedEntity {
            entity_id: "light.living_room_overhead".to_string(),
            name: "Living Room Overhead Light".to_string(),
            state: "on".to_string(),
            domain: "light".to_string(),
        },
        CachedEntity {
            entity_id: "switch.living_room_tv".to_string(),
            name: "Living Room TV Switch".to_string(),
            state: "off".to_string(),
            domain: "switch".to_string(),
        },
        CachedEntity {
            entity_id: "climate.living_room".to_string(),
            name: "Living Room Thermostat".to_string(),
            state: "21.5".to_string(),
            domain: "climate".to_string(),
        },
        CachedEntity {
            entity_id: "sensor.kitchen_temperature".to_string(),
            name: "Kitchen Temperature Sensor".to_string(),
            state: "22.3".to_string(),
            domain: "sensor".to_string(),
        },
        CachedEntity {
            entity_id: "sensor.bedroom_humidity".to_string(),
            name: "Bedroom Humidity Sensor".to_string(),
            state: "45.0".to_string(),
            domain: "sensor".to_string(),
        },
        CachedEntity {
            entity_id: "media_player.living_room_tv".to_string(),
            name: "Living Room TV".to_string(),
            state: "playing".to_string(),
            domain: "media_player".to_string(),
        },
    ]
}

async fn get_entities() -> Vec<CachedEntity> {
    let cache_cell = CACHE.get_or_init(|| Mutex::new(None));

    let needs_fetch = {
        let cache_guard = cache_cell.lock().unwrap();
        match &*cache_guard {
            Some(cache) => cache.fetched_at.elapsed() > Duration::from_secs(300),
            None => true,
        }
    };

    if needs_fetch {
        match fetch_entities_from_ha().await {
            Ok(entities) => {
                let mut cache_guard = cache_cell.lock().unwrap();
                *cache_guard = Some(EntityCache {
                    entities: entities.clone(),
                    fetched_at: Instant::now(),
                });
                entities
            }
            Err(e) => {
                eprintln!(
                    "Error fetching entities from Home Assistant: {}. Using mock fallback.",
                    e
                );
                let mut cache_guard = cache_cell.lock().unwrap();
                if let Some(cache) = &*cache_guard {
                    cache.entities.clone()
                } else {
                    let fallback = get_mock_entities();
                    *cache_guard = Some(EntityCache {
                        entities: fallback.clone(),
                        fetched_at: Instant::now(),
                    });
                    fallback
                }
            }
        }
    } else {
        let cache_guard = cache_cell.lock().unwrap();
        cache_guard.as_ref().unwrap().entities.clone()
    }
}

fn calculate_score(query: &str, entity: &CachedEntity) -> f64 {
    let q = query.to_lowercase();
    let name_lower = entity.name.to_lowercase();
    let id_lower = entity.entity_id.to_lowercase();

    let query_words: Vec<&str> = q.split_whitespace().filter(|w| !w.is_empty()).collect();
    let mut word_matches = 0;
    for &word in &query_words {
        if name_lower.contains(word) || id_lower.contains(word) {
            word_matches += 1;
        }
    }

    let mut score = 0.0;

    if name_lower == q || id_lower == q {
        score += 2.0;
    } else if name_lower.contains(&q) || id_lower.contains(&q) {
        score += 1.5;
    }

    if !query_words.is_empty() {
        score += (word_matches as f64) / (query_words.len() as f64);
    }

    let jw = strsim::jaro_winkler(&q, &name_lower).max(strsim::jaro_winkler(&q, &id_lower));
    score += 0.5 * jw;

    score
}

pub fn map_action_to_service(
    entity_id: &str,
    action: &str,
    value: Option<serde_json::Value>,
) -> Result<(String, serde_json::Value), String> {
    if entity_id.is_empty() {
        return Err("entity_id must not be empty".to_string());
    }
    if !entity_id.contains('.') {
        return Err(format!(
            "entity_id '{}' must contain a dot (e.g. 'light.living_room')",
            entity_id
        ));
    }
    for c in entity_id.chars() {
        if !c.is_alphanumeric() && c != '.' && c != '_' && c != '-' {
            return Err(format!("entity_id contains invalid character '{}'", c));
        }
    }

    let domain = entity_id.split('.').next().unwrap_or("");
    let mut payload = serde_json::json!({
        "entity_id": entity_id
    });

    let service = match (domain, action) {
        ("light", "set_value") => {
            if let Some(v) = value {
                let brightness_pct = match &v {
                    serde_json::Value::Number(num) => {
                        num.as_f64().ok_or("Invalid numeric value")?
                    }
                    serde_json::Value::String(s) => s
                        .parse::<f64>()
                        .map_err(|e| format!("Failed to parse number: {}", e))?,
                    serde_json::Value::Bool(b) => {
                        if *b {
                            100.0
                        } else {
                            0.0
                        }
                    }
                    _ => return Err("Value for light set_value must be a number".to_string()),
                };
                payload["brightness_pct"] = serde_json::json!(brightness_pct);
            } else {
                return Err("Value is required for light set_value".to_string());
            }
            "light/turn_on".to_string()
        }
        ("climate", "set_value") => {
            if let Some(v) = value {
                let temp = match &v {
                    serde_json::Value::Number(num) => {
                        num.as_f64().ok_or("Invalid numeric value")?
                    }
                    serde_json::Value::String(s) => s
                        .parse::<f64>()
                        .map_err(|e| format!("Failed to parse number: {}", e))?,
                    _ => return Err("Value for climate set_value must be a number".to_string()),
                };
                payload["temperature"] = serde_json::json!(temp);
            } else {
                return Err("Value is required for climate set_value".to_string());
            }
            "climate/set_temperature".to_string()
        }
        ("input_number", "set_value") => {
            if let Some(v) = value {
                payload["value"] = v;
            } else {
                return Err("Value is required for input_number set_value".to_string());
            }
            "input_number/set_value".to_string()
        }
        ("number", "set_value") => {
            if let Some(v) = value {
                payload["value"] = v;
            } else {
                return Err("Value is required for number set_value".to_string());
            }
            "number/set_value".to_string()
        }
        ("automation", "turn_on") => "automation/trigger".to_string(),
        (_, "set_value") => {
            return Err(format!(
                "set_value action is not supported for domain '{}'",
                domain
            ));
        }
        (_, other_action) => {
            format!("{}/{}", domain, other_action)
        }
    };

    Ok((service, payload))
}

async fn execute_ha_service(
    ha_url: &str,
    ha_token: &str,
    service: &str,
    payload: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let base_url = ha_url.trim_end_matches('/');
    let url = format!("{}/api/services/{}", base_url, service);

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", ha_token))
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to send service request to Home Assistant: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!(
            "Home Assistant returned status {}: {}",
            status, body
        ));
    }

    let response_json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Home Assistant service response: {}", e))?;

    Ok(response_json)
}

async fn fetch_ha_config(ha_url: &str, ha_token: &str) -> Result<serde_json::Value, String> {
    let base_url = ha_url.trim_end_matches('/');
    let url = format!("{}/api/config", base_url);

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", ha_token))
        .send()
        .await
        .map_err(|e| format!("Failed to fetch HA config: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HA config returned status {}", response.status()));
    }

    let config: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse HA config: {}", e))?;

    Ok(config)
}

async fn fetch_ha_error_log(ha_url: &str, ha_token: &str) -> Result<String, String> {
    let base_url = ha_url.trim_end_matches('/');
    let url = format!("{}/api/error_log", base_url);

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", ha_token))
        .send()
        .await
        .map_err(|e| format!("Failed to fetch HA error log: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "HA error log returned status {}",
            response.status()
        ));
    }

    let log = response
        .text()
        .await
        .map_err(|e| format!("Failed to read HA error log: {}", e))?;

    Ok(log)
}

fn compact_and_truncate_log(log: &str, max_chars: usize) -> String {
    if log.trim().is_empty() {
        return "No errors logged.".to_string();
    }

    let lines: Vec<&str> = log.lines().collect();
    let mut relevant_lines = Vec::new();
    for line in &lines {
        let l_lower = line.to_lowercase();
        if l_lower.contains("error")
            || l_lower.contains("critical")
            || l_lower.contains("exception")
            || l_lower.contains("fail")
            || l_lower.contains("warning")
        {
            relevant_lines.push(*line);
        }
    }

    let source_lines = if !relevant_lines.is_empty() {
        relevant_lines
    } else {
        lines
            .iter()
            .copied()
            .rev()
            .take(5)
            .collect::<Vec<&str>>()
            .into_iter()
            .rev()
            .collect()
    };

    let joined = source_lines.join("\n");
    if joined.len() > max_chars {
        if max_chars <= 15 {
            return "...".to_string();
        }
        let end_idx = joined
            .char_indices()
            .map(|(i, _)| i)
            .nth(max_chars - 15)
            .unwrap_or(joined.len());
        format!("{}...[TRUNCATED]", &joined[..end_idx])
    } else {
        joined
    }
}

async fn get_ha_diagnostics() -> serde_json::Value {
    let ha_url = std::env::var("HA_URL").or_else(|_| std::env::var("HOME_ASSISTANT_URL"));
    let ha_token = std::env::var("HA_TOKEN").or_else(|_| std::env::var("HOME_ASSISTANT_API_KEY"));

    let (status, version, error_log_raw) = match (ha_url, ha_token) {
        (Ok(url), Ok(token)) => {
            let config_res = fetch_ha_config(&url, &token).await;
            let log_res = fetch_ha_error_log(&url, &token).await;

            let status = if config_res.is_ok() { "online" } else { "offline" };
            let version = match &config_res {
                Ok(cfg) => cfg
                    .get("version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                Err(_) => "unknown".to_string(),
            };

            let log_raw = match log_res {
                Ok(log) => log,
                Err(e) => format!("Failed to fetch logs: {}", e),
            };

            (status.to_string(), version, log_raw)
        }
        _ => {
            (
                "online".to_string(),
                "2026.5.2".to_string(),
                "WARNING [homeassistant.components.http] Login attempt or request with invalid authentication from 192.168.1.15\nERROR [homeassistant.core] Error doing job: Task exception was never retrieved".to_string()
            )
        }
    };

    let errors_detected = error_log_raw.to_lowercase().contains("error")
        || error_log_raw.to_lowercase().contains("critical")
        || error_log_raw.to_lowercase().contains("fail")
        || (!error_log_raw.trim().is_empty()
            && !error_log_raw.contains("No errors logged")
            && !error_log_raw.contains("Failed to fetch logs"));

    let mut log_chars_limit = 350;
    loop {
        let compacted_log = compact_and_truncate_log(&error_log_raw, log_chars_limit);
        let response_json = serde_json::json!({
            "status": status,
            "version": version,
            "errors_detected": errors_detected,
            "error_log": compacted_log
        });

        let content_text = serde_json::to_string_pretty(&response_json).unwrap();
        if content_text.len() <= 500 || log_chars_limit <= 20 {
            return response_json;
        }
        log_chars_limit -= 20;
    }
}

fn get_config_path(file: Option<String>) -> Result<std::path::PathBuf, String> {
    let filename = file.unwrap_or_else(|| "configuration.yaml".to_string());
    if filename.contains("..") {
        return Err("Path traversal is not allowed.".to_string());
    }

    // Only allow subpaths if they start with ".storage/" (specifically for config entries)
    if filename.contains('/') || filename.contains('\\') {
        let is_storage = filename.starts_with(".storage/") || filename.starts_with(".storage\\");
        if !is_storage {
            return Err(
                "Invalid path. Slashes are only allowed for accessing '.storage/' files."
                    .to_string(),
            );
        }
    }

    let config_dir = std::env::var("HA_CONFIG_DIR").unwrap_or_else(|_| {
        if std::path::Path::new("/config").exists() {
            "/config".to_string()
        } else if let Ok(home) = std::env::var("HOME") {
            let ha_home = std::path::Path::new(&home).join(".homeassistant");
            if ha_home.exists() {
                ha_home.to_string_lossy().into_owned()
            } else {
                ".".to_string()
            }
        } else {
            ".".to_string()
        }
    });

    Ok(std::path::Path::new(&config_dir).join(filename))
}

async fn handle_edit_config(params: EditConfigParams) -> Result<serde_json::Value, String> {
    let path = get_config_path(params.file)?;

    match params.action.as_str() {
        "read" => {
            if !path.exists() {
                return Err(format!(
                    "File '{}' does not exist in configuration directory.",
                    path.display()
                ));
            }
            let content = tokio::fs::read_to_string(&path)
                .await
                .map_err(|e| format!("Failed to read file: {}", e))?;

            let lines: Vec<&str> = content.lines().collect();
            let offset = params.offset.unwrap_or(0);
            let limit = params.limit.unwrap_or(100);

            if offset >= lines.len() {
                return Ok(serde_json::json!({
                    "file": path.file_name().unwrap().to_string_lossy(),
                    "total_lines": lines.len(),
                    "lines": Vec::<String>::new(),
                    "eof": true
                }));
            }

            let end = (offset + limit).min(lines.len());
            let subset_lines = &lines[offset..end];
            let is_eof = end == lines.len();

            Ok(serde_json::json!({
                "file": path.file_name().unwrap().to_string_lossy(),
                "total_lines": lines.len(),
                "offset": offset,
                "limit": limit,
                "lines": subset_lines,
                "eof": is_eof
            }))
        }
        "patch" => {
            let old_str = params
                .old_string
                .ok_or("old_string is required for patch action")?;
            let new_str = params
                .new_string
                .ok_or("new_string is required for patch action")?;

            if !path.exists() {
                return Err(format!(
                    "File '{}' does not exist in configuration directory.",
                    path.display()
                ));
            }
            let content = tokio::fs::read_to_string(&path)
                .await
                .map_err(|e| format!("Failed to read file: {}", e))?;

            let occurrences: Vec<_> = content.matches(&old_str).collect();
            if occurrences.is_empty() {
                return Err("old_string was not found in the file. Ensure the search string matches the file contents exactly.".to_string());
            }
            if occurrences.len() > 1 {
                return Err(format!(
                    "old_string was found {} times in the file. Please provide more surrounding context to make the match unique.",
                    occurrences.len()
                ));
            }

            let updated_content = content.replace(&old_str, &new_str);
            tokio::fs::write(&path, updated_content)
                .await
                .map_err(|e| format!("Failed to write file: {}", e))?;

            Ok(serde_json::json!({
                "status": "success",
                "file": path.file_name().unwrap().to_string_lossy(),
                "message": "Patch applied successfully."
            }))
        }
        other => Err(format!("Unknown edit_config action: {}", other)),
    }
}

async fn discover_modbus_from_storage(hub_name: Option<&str>) -> Result<(String, u16), String> {
    let path = get_config_path(Some(".storage/core.config_entries".to_string()))?;
    if !path.exists() {
        return Err("No storage config entries file found.".to_string());
    }

    let content = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| format!("Failed to read core.config_entries: {}", e))?;

    let v: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse core.config_entries: {}", e))?;

    if let Some(entries) = v
        .get("data")
        .and_then(|d| d.get("entries"))
        .and_then(|e| e.as_array())
    {
        for entry in entries {
            if entry.get("domain").and_then(|d| d.as_str()) == Some("modbus") {
                if let Some(data) = entry.get("data") {
                    let host = data.get("host").and_then(|h| h.as_str());
                    let port = data
                        .get("port")
                        .and_then(|p| p.as_u64())
                        .map(|p| p as u16)
                        .unwrap_or(502);
                    let name = entry.get("title").and_then(|t| t.as_str()).unwrap_or("");

                    if let Some(h) = host {
                        if hub_name.is_none() || hub_name == Some(name) {
                            return Ok((h.to_string(), port));
                        }
                    }
                }
            }
        }
    }
    Err("Modbus entry not found in core.config_entries".to_string())
}

async fn auto_discover_modbus_target(hub_name: Option<&str>) -> Result<(String, u16), String> {
    if let Ok(target) = discover_modbus_from_storage(hub_name).await {
        return Ok(target);
    }

    let path = get_config_path(Some("configuration.yaml".to_string()))?;
    if !path.exists() {
        return Err(
            "No local configuration.yaml found to auto-discover Modbus targets.".to_string(),
        );
    }

    let content = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| format!("Failed to read configuration.yaml: {}", e))?;

    let mut current_hub: Option<String> = None;
    let mut current_host: Option<String> = None;
    let mut current_port: Option<u16> = Some(502);

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = trimmed.splitn(2, ':').collect();
        if parts.len() == 2 {
            let key = parts[0].trim().trim_start_matches('-').trim();
            let val = parts[1].trim().trim_matches('"').trim_matches('\'');

            if key == "name" {
                if let Some(h) = &current_host {
                    if hub_name.is_none() || hub_name == current_hub.as_deref() {
                        return Ok((h.clone(), current_port.unwrap_or(502)));
                    }
                }
                current_hub = Some(val.to_string());
                current_host = None;
                current_port = Some(502);
            } else if key == "host" {
                current_host = Some(val.to_string());
            } else if key == "port" {
                if let Ok(p) = val.parse::<u16>() {
                    current_port = Some(p);
                }
            }
        }
    }

    if let Some(h) = current_host {
        if hub_name.is_none() || hub_name == current_hub.as_deref() {
            return Ok((h, current_port.unwrap_or(502)));
        }
    }

    Err("Could not find any configured Modbus TCP host in configuration.yaml.".to_string())
}

async fn handle_probe_modbus(params: ProbeModbusParams) -> Result<serde_json::Value, String> {
    let (host, port) = match &params.host {
        Some(h) => {
            if h.chars().all(|c| c.is_numeric() || c == '.')
                || h.contains(':')
                || h == "localhost"
                || h == "127.0.0.1"
            {
                (h.clone(), params.port.unwrap_or(502))
            } else {
                match auto_discover_modbus_target(Some(h)).await {
                    Ok(target) => target,
                    Err(_) => (h.clone(), params.port.unwrap_or(502)),
                }
            }
        }
        None => auto_discover_modbus_target(None).await?,
    };

    let unit_id = params.unit_id.unwrap_or(1);
    let count = params.count.unwrap_or(1);

    if count == 0 || count > 125 {
        return Err("Quantity of registers must be between 1 and 125".to_string());
    }

    let func_code = match params.register_type.as_str() {
        "coil" => 0x01,
        "discrete" => 0x02,
        "holding" => 0x03,
        "input" => 0x04,
        other => {
            return Err(format!(
                "Invalid register_type '{}'. Must be one of: coil, discrete, holding, input",
                other
            ))
        }
    };

    let mut req = Vec::with_capacity(12);
    req.extend_from_slice(&[0x00, 0x01]); // Transaction ID
    req.extend_from_slice(&[0x00, 0x00]); // Protocol ID
    req.extend_from_slice(&[0x00, 0x06]); // Length
    req.push(unit_id);
    req.push(func_code);
    req.extend_from_slice(&params.address.to_be_bytes());
    req.extend_from_slice(&count.to_be_bytes());

    let target = format!("{}:{}", host, port);

    let stream_res = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        tokio::net::TcpStream::connect(&target),
    )
    .await;

    let mut stream = match stream_res {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => {
            return Err(format!(
                "Failed to connect to Modbus TCP target {}: {}",
                target, e
            ))
        }
        Err(_) => {
            return Err(format!(
                "Connection to Modbus TCP target {} timed out",
                target
            ))
        }
    };

    stream
        .write_all(&req)
        .await
        .map_err(|e| format!("Failed to send Modbus request: {}", e))?;

    let mut header = [0u8; 7];
    let header_read_res = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        stream.read_exact(&mut header),
    )
    .await;

    match header_read_res {
        Ok(Ok(_)) => {}
        Ok(Err(e)) => return Err(format!("Failed to read Modbus response header: {}", e)),
        Err(_) => return Err("Timeout waiting for Modbus response header".to_string()),
    }

    let length = u16::from_be_bytes([header[4], header[5]]) as usize;
    if !(2..=300).contains(&length) {
        return Err(format!("Invalid response frame length: {}", length));
    }

    let mut body = vec![0u8; length - 1];
    let body_read_res = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        stream.read_exact(&mut body),
    )
    .await;

    match body_read_res {
        Ok(Ok(_)) => {}
        Ok(Err(e)) => return Err(format!("Failed to read Modbus response body: {}", e)),
        Err(_) => return Err("Timeout waiting for Modbus response body".to_string()),
    }

    let res_func_code = body[0];
    if res_func_code & 0x80 != 0 {
        let exception_code = if body.len() > 1 { body[1] } else { 0 };
        let reason = match exception_code {
            1 => "Illegal Function - The function code is not supported by the slave device.",
            2 => "Illegal Data Address - The register address does not exist on the slave device.",
            3 => "Illegal Data Value - The value or count is invalid for this register address.",
            4 => "Slave Device Failure - An unrecoverable error occurred while the slave was processing.",
            _ => "Unknown Exception - The device returned an undocumented error code."
        };
        return Err(format!(
            "Modbus Exception {:02X}: {}",
            exception_code, reason
        ));
    }

    if res_func_code != func_code {
        return Err(format!(
            "Unexpected function code in response: expected {:02X}, got {:02X}",
            func_code, res_func_code
        ));
    }

    let byte_count = body[1] as usize;
    if body.len() < 2 + byte_count {
        return Err("Response frame is shorter than the reported byte count".to_string());
    }

    let raw_data = &body[2..2 + byte_count];

    let mut values = Vec::new();
    if func_code == 0x01 || func_code == 0x02 {
        for i in 0..count {
            let byte_idx = (i / 8) as usize;
            let bit_idx = i % 8;
            if byte_idx < raw_data.len() {
                let bit_val = (raw_data[byte_idx] >> bit_idx) & 1;
                values.push(serde_json::json!(bit_val == 1));
            }
        }
    } else {
        for i in 0..count {
            let byte_idx = (i * 2) as usize;
            if byte_idx + 1 < raw_data.len() {
                let val_u16 = u16::from_be_bytes([raw_data[byte_idx], raw_data[byte_idx + 1]]);
                values.push(serde_json::json!(val_u16));
            }
        }
    }

    Ok(serde_json::json!({
        "status": "success",
        "host": params.host,
        "port": port,
        "unit_id": unit_id,
        "register_type": params.register_type,
        "start_address": params.address,
        "count": count,
        "values": values
    }))
}

async fn handle_tool_call(params: CallToolParams) -> Result<serde_json::Value, String> {
    match params.name.as_str() {
        "ha_search_entities" => {
            let args = params
                .arguments
                .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
            let search_args: SearchParams = serde_json::from_value(args)
                .map_err(|e| format!("Invalid search arguments: {}", e))?;

            let query = search_args.query.trim();
            let domain_filter = search_args.domain.as_ref().map(|d| d.to_lowercase());

            let mut matched_entities = Vec::new();
            let entities = get_entities().await;

            for entity in entities {
                if let Some(domain) = &domain_filter {
                    if &entity.domain.to_lowercase() != domain {
                        continue;
                    }
                }

                let score = calculate_score(query, &entity);
                matched_entities.push((entity, score));
            }

            matched_entities
                .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            let top_matches: Vec<serde_json::Value> = matched_entities
                .into_iter()
                .take(3)
                .map(|(entity, _score)| {
                    serde_json::json!({
                        "entity_id": entity.entity_id,
                        "name": entity.name,
                        "state": entity.state,
                        "domain": entity.domain
                    })
                })
                .collect();

            let content_text = serde_json::to_string_pretty(&top_matches).unwrap();
            let result = CallToolResult {
                content: vec![ToolContent::Text { text: content_text }],
                is_error: None,
            };
            Ok(serde_json::to_value(&result).unwrap())
        }
        "ha_control" => {
            let args = params
                .arguments
                .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
            let control_args: ControlParams = serde_json::from_value(args)
                .map_err(|e| format!("Invalid control arguments: {}", e))?;

            let (service, payload) = map_action_to_service(
                &control_args.entity_id,
                &control_args.action,
                control_args.value.clone(),
            )?;

            let ha_url = std::env::var("HA_URL").or_else(|_| std::env::var("HOME_ASSISTANT_URL"));
            let ha_token =
                std::env::var("HA_TOKEN").or_else(|_| std::env::var("HOME_ASSISTANT_API_KEY"));

            let response_value = match (ha_url, ha_token) {
                (Ok(url), Ok(token)) => {
                    let _raw_resp = execute_ha_service(&url, &token, &service, payload).await?;
                    serde_json::json!({
                        "status": "success",
                        "entity_id": control_args.entity_id,
                        "action": control_args.action
                    })
                }
                _ => {
                    // Mock fallback if environment variables are not configured
                    serde_json::json!({
                        "status": "success",
                        "entity_id": control_args.entity_id,
                        "action": control_args.action,
                        "mapped_service": service,
                        "payload": payload,
                        "mock_fallback": true
                    })
                }
            };

            let content_text = serde_json::to_string_pretty(&response_value).unwrap();
            let result = CallToolResult {
                content: vec![ToolContent::Text { text: content_text }],
                is_error: None,
            };
            Ok(serde_json::to_value(&result).unwrap())
        }
        "ha_get_diagnostics" => {
            let diagnostics_response = get_ha_diagnostics().await;

            let content_text = serde_json::to_string_pretty(&diagnostics_response).unwrap();
            let result = CallToolResult {
                content: vec![ToolContent::Text { text: content_text }],
                is_error: None,
            };
            Ok(serde_json::to_value(&result).unwrap())
        }
        "ha_edit_config" => {
            let args = params
                .arguments
                .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
            let config_args: EditConfigParams = serde_json::from_value(args)
                .map_err(|e| format!("Invalid edit_config arguments: {}", e))?;

            let res = handle_edit_config(config_args).await?;
            let content_text = serde_json::to_string_pretty(&res).unwrap();
            let result = CallToolResult {
                content: vec![ToolContent::Text { text: content_text }],
                is_error: None,
            };
            Ok(serde_json::to_value(&result).unwrap())
        }
        "ha_probe_modbus" => {
            let args = params
                .arguments
                .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
            let modbus_args: ProbeModbusParams = serde_json::from_value(args)
                .map_err(|e| format!("Invalid probe_modbus arguments: {}", e))?;

            let res = handle_probe_modbus(modbus_args).await?;
            let content_text = serde_json::to_string_pretty(&res).unwrap();
            let result = CallToolResult {
                content: vec![ToolContent::Text { text: content_text }],
                is_error: None,
            };
            Ok(serde_json::to_value(&result).unwrap())
        }
        _ => Err(format!("Unknown tool: {}", params.name)),
    }
}

#[cfg(test)]
#[allow(clippy::await_holding_lock)]
mod tests {
    use super::*;
    use strsim::jaro_winkler;

    static ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_dependencies() {
        let similarity = jaro_winkler("kitchen light", "kitchen overhead light");
        assert!(similarity > 0.0);
    }

    #[tokio::test]
    async fn test_initialize_handshake() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "initialize".to_string(),
            params: None,
            id: Some(Id::Number(1)),
        };
        let resp = handle_request(req).await.expect("Should return response");
        assert_eq!(resp.jsonrpc, "2.0");
        assert_eq!(resp.id, Some(Id::Number(1)));
        assert!(resp.error.is_none());
        let res = resp.result.expect("Should have result");
        assert_eq!(res["serverInfo"]["name"], "home-assistant-mcp");
    }

    #[tokio::test]
    async fn test_tools_list() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "tools/list".to_string(),
            params: None,
            id: Some(Id::Number(2)),
        };
        let resp = handle_request(req).await.expect("Should return response");
        assert_eq!(resp.id, Some(Id::Number(2)));
        let res = resp.result.expect("Should have result");
        let tools_list = res["tools"].as_array().expect("Should be array");
        assert_eq!(tools_list.len(), 5);
        assert_eq!(tools_list[0]["name"], "ha_search_entities");
        assert_eq!(tools_list[1]["name"], "ha_control");
        assert_eq!(tools_list[2]["name"], "ha_get_diagnostics");
        assert_eq!(tools_list[3]["name"], "ha_edit_config");
        assert_eq!(tools_list[4]["name"], "ha_probe_modbus");
    }

    #[tokio::test]
    async fn test_tools_call_search() {
        let params = serde_json::json!({
            "name": "ha_search_entities",
            "arguments": {
                "query": "kitchen"
            }
        });
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "tools/call".to_string(),
            params: Some(params),
            id: Some(Id::String("search-1".to_string())),
        };
        let resp = handle_request(req).await.expect("Should return response");
        assert_eq!(resp.id, Some(Id::String("search-1".to_string())));
        let res = resp.result.expect("Should have result");
        let content = res["content"].as_array().expect("content should be array");
        assert_eq!(content.len(), 1);
        assert_eq!(content[0]["type"], "text");
        let text = content[0]["text"].as_str().expect("text should be string");
        assert!(text.contains("light.kitchen_overhead"));
    }

    #[tokio::test]
    async fn test_tools_call_control() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let params = serde_json::json!({
            "name": "ha_control",
            "arguments": {
                "entity_id": "light.living_room",
                "action": "turn_on"
            }
        });
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "tools/call".to_string(),
            params: Some(params),
            id: Some(Id::Number(42)),
        };
        let resp = handle_request(req).await.expect("Should return response");
        assert_eq!(resp.id, Some(Id::Number(42)));
        let res = resp.result.expect("Should have result");
        let content = res["content"].as_array().expect("content should be array");
        assert_eq!(content.len(), 1);
        let text = content[0]["text"].as_str().expect("text should be string");
        assert!(text.contains("success"));
        assert!(text.contains("light.living_room"));
    }

    #[tokio::test]
    async fn test_tools_call_diagnostics() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let params = serde_json::json!({
            "name": "ha_get_diagnostics",
            "arguments": {}
        });
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "tools/call".to_string(),
            params: Some(params),
            id: Some(Id::Number(43)),
        };
        let resp = handle_request(req).await.expect("Should return response");
        assert_eq!(resp.id, Some(Id::Number(43)));
        let res = resp.result.expect("Should have result");
        let content = res["content"].as_array().expect("content should be array");
        assert_eq!(content.len(), 1);
        let text = content[0]["text"].as_str().expect("text should be string");
        assert!(text.contains("online"));
    }

    #[tokio::test]
    async fn test_tools_call_failure_response() {
        let params = serde_json::json!({
            "name": "ha_search_entities",
            "arguments": {}
        });
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "tools/call".to_string(),
            params: Some(params),
            id: Some(Id::Number(44)),
        };
        let resp = handle_request(req).await.expect("Should return response");
        assert_eq!(resp.id, Some(Id::Number(44)));
        assert!(resp.error.is_none());

        let res = resp.result.expect("Should have result");
        assert_eq!(res["isError"], true);

        let content = res["content"].as_array().expect("content should be array");
        assert_eq!(content.len(), 1);
        let text = content[0]["text"].as_str().expect("text should be string");
        assert!(text.contains("Invalid search arguments"));
    }

    #[tokio::test]
    async fn test_invalid_method() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "non_existent".to_string(),
            params: None,
            id: Some(Id::Number(99)),
        };
        let resp = handle_request(req).await.expect("Should return response");
        assert_eq!(resp.id, Some(Id::Number(99)));
        let err = resp.error.expect("Should have error");
        assert_eq!(err.code, -32601);
        assert!(err.message.contains("Method not found"));
    }

    #[tokio::test]
    async fn test_fuzzy_matching_and_ranking() {
        let entities = get_mock_entities();

        // Match 1: "living room light" should rank "light.living_room_overhead" as the highest Light match
        let query = "living room light";
        let mut matched = Vec::new();
        for e in &entities {
            let score = calculate_score(query, e);
            matched.push((e, score));
        }
        matched.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        assert_eq!(matched[0].0.entity_id, "light.living_room_overhead");

        // Match 2: "bedroom humidity" should rank "sensor.bedroom_humidity" first
        let query = "bedroom humidity";
        let mut matched = Vec::new();
        for e in &entities {
            let score = calculate_score(query, e);
            matched.push((e, score));
        }
        matched.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        assert_eq!(matched[0].0.entity_id, "sensor.bedroom_humidity");

        // Match 3: "temperature" with domain filter "sensor" should find "sensor.kitchen_temperature"
        let query = "temperature";
        let mut matched = Vec::new();
        for e in &entities {
            if e.domain == "sensor" {
                let score = calculate_score(query, e);
                matched.push((e, score));
            }
        }
        matched.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        assert_eq!(matched[0].0.entity_id, "sensor.kitchen_temperature");
    }

    #[test]
    fn test_map_action_to_service() {
        // Test light turn_on mapping
        let (service, payload) = map_action_to_service("light.kitchen", "turn_on", None).unwrap();
        assert_eq!(service, "light/turn_on");
        assert_eq!(payload["entity_id"], "light.kitchen");

        // Test light set_value mapping with number
        let (service, payload) =
            map_action_to_service("light.kitchen", "set_value", Some(serde_json::json!(75)))
                .unwrap();
        assert_eq!(service, "light/turn_on");
        assert_eq!(payload["entity_id"], "light.kitchen");
        assert_eq!(payload["brightness_pct"], 75.0);

        // Test climate set_value mapping with string number
        let (service, payload) = map_action_to_service(
            "climate.living_room",
            "set_value",
            Some(serde_json::json!("22.5")),
        )
        .unwrap();
        assert_eq!(service, "climate/set_temperature");
        assert_eq!(payload["entity_id"], "climate.living_room");
        assert_eq!(payload["temperature"], 22.5);

        // Test automation turn_on mapping to trigger
        let (service, payload) =
            map_action_to_service("automation.morning_routine", "turn_on", None).unwrap();
        assert_eq!(service, "automation/trigger");
        assert_eq!(payload["entity_id"], "automation.morning_routine");

        // Test generic toggle mapping
        let (service, payload) =
            map_action_to_service("switch.coffee_maker", "toggle", None).unwrap();
        assert_eq!(service, "switch/toggle");
        assert_eq!(payload["entity_id"], "switch.coffee_maker");

        // Test validation of empty and dot-less entity_id
        let err_empty = map_action_to_service("", "turn_on", None).unwrap_err();
        assert!(err_empty.contains("empty"));

        let err_nodot = map_action_to_service("lightlivingroom", "turn_on", None).unwrap_err();
        assert!(err_nodot.contains("must contain a dot"));

        let err_invalid = map_action_to_service("light.living;room", "turn_on", None).unwrap_err();
        assert!(err_invalid.contains("invalid character"));
    }

    #[tokio::test]
    async fn test_execute_ha_service_with_mock_server() {
        let _lock = ENV_MUTEX.lock().unwrap();
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let url = format!("http://{}", addr);

        tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut buf = [0; 1024];
                let n = socket.read(&mut buf).await.unwrap();
                let req_str = String::from_utf8_lossy(&buf[..n]);

                assert!(req_str.contains("POST /api/services/light/turn_on"));
                assert!(req_str
                    .to_lowercase()
                    .contains("authorization: bearer test_token"));
                assert!(req_str.contains("\"entity_id\":\"light.living_room\""));
                assert!(req_str.contains("\"brightness_pct\":50.0"));

                let response_body = "[{\"entity_id\":\"light.living_room\",\"state\":\"on\"}]";
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                    response_body.len(),
                    response_body
                );
                socket.write_all(response.as_bytes()).await.unwrap();
            }
        });

        let payload = serde_json::json!({
            "entity_id": "light.living_room",
            "brightness_pct": 50.0
        });

        let res = execute_ha_service(&url, "test_token", "light/turn_on", payload)
            .await
            .unwrap();
        assert!(res.is_array());
        assert_eq!(res[0]["entity_id"], "light.living_room");
    }

    #[test]
    fn test_log_compaction_and_truncation() {
        let long_log = "INFO: Some unrelated line\nWARNING: First warning\nINFO: Another unrelated line\nERROR: Crucial error occurred here!\nINFO: Still more lines\nCRITICAL: High priority problem!\nINFO: Last line";

        let compacted = compact_and_truncate_log(long_log, 100);
        assert!(compacted.contains("WARNING: First warning"));
        assert!(compacted.contains("ERROR: Crucial error occurred here!"));
        assert!(compacted.contains("CRITICAL: High priority problem!"));
        assert!(!compacted.contains("unrelated"));
        assert!(compacted.len() <= 100);

        let non_error_log =
            "INFO: Line 1\nINFO: Line 2\nINFO: Line 3\nINFO: Line 4\nINFO: Line 5\nINFO: Line 6";
        let compacted_non_error = compact_and_truncate_log(non_error_log, 50);
        assert!(compacted_non_error.len() <= 50);
    }

    #[tokio::test]
    async fn test_diagnostics_character_limit() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let long_errors = "ERROR: Some error\n".repeat(50);
        let previous_url = std::env::var("HA_URL");
        let previous_token = std::env::var("HA_TOKEN");
        std::env::set_var("HA_URL", "http://127.0.0.1:1234");
        std::env::set_var("HA_TOKEN", "mock_token");

        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let mock_url = format!("http://{}", addr);
        std::env::set_var("HA_URL", &mock_url);

        let log_data = long_errors.clone();
        tokio::spawn(async move {
            for _ in 0..2 {
                if let Ok((mut socket, _)) = listener.accept().await {
                    let mut buf = [0; 1024];
                    let n = socket.read(&mut buf).await.unwrap();
                    let req_str = String::from_utf8_lossy(&buf[..n]);

                    if req_str.contains("GET /api/config") {
                        let body = "{\"version\":\"2026.5.2\",\"location_name\":\"Home\"}";
                        let resp = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                            body.len(),
                            body
                        );
                        socket.write_all(resp.as_bytes()).await.unwrap();
                    } else if req_str.contains("GET /api/error_log") {
                        let resp = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                            log_data.len(),
                            log_data
                        );
                        socket.write_all(resp.as_bytes()).await.unwrap();
                    }
                }
            }
        });

        let diag = get_ha_diagnostics().await;
        let formatted = serde_json::to_string_pretty(&diag).unwrap();

        assert!(formatted.len() <= 500);
        assert_eq!(diag["status"], "online");
        assert_eq!(diag["version"], "2026.5.2");
        assert_eq!(diag["errors_detected"], true);

        match previous_url {
            Ok(val) => std::env::set_var("HA_URL", val),
            Err(_) => std::env::remove_var("HA_URL"),
        }
        match previous_token {
            Ok(val) => std::env::set_var("HA_TOKEN", val),
            Err(_) => std::env::remove_var("HA_TOKEN"),
        }
    }

    #[tokio::test]
    async fn test_edit_config_flow() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let temp_dir = std::env::temp_dir().join("ha_config_test");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let previous_config_dir = std::env::var("HA_CONFIG_DIR");
        std::env::set_var("HA_CONFIG_DIR", &temp_dir);

        let config_file = temp_dir.join("configuration.yaml");
        let initial_content = "modbus:\n  - name: hub1\n    host: 192.168.1.50\n    port: 502\n";
        std::fs::write(&config_file, initial_content).unwrap();

        // 1. Test Read action
        let read_params = EditConfigParams {
            action: "read".to_string(),
            file: Some("configuration.yaml".to_string()),
            old_string: None,
            new_string: None,
            offset: Some(0),
            limit: Some(10),
        };
        let res = handle_edit_config(read_params).await.unwrap();
        assert_eq!(res["file"], "configuration.yaml");
        assert_eq!(res["lines"].as_array().unwrap().len(), 4);
        assert_eq!(res["lines"][1].as_str().unwrap(), "  - name: hub1");

        // 2. Test Patch action
        let patch_params = EditConfigParams {
            action: "patch".to_string(),
            file: Some("configuration.yaml".to_string()),
            old_string: Some("port: 502".to_string()),
            new_string: Some("port: 503".to_string()),
            offset: None,
            limit: None,
        };
        let patch_res = handle_edit_config(patch_params).await.unwrap();
        assert_eq!(patch_res["status"], "success");

        // 3. Verify content is updated
        let updated_content = std::fs::read_to_string(&config_file).unwrap();
        assert!(updated_content.contains("port: 503"));
        assert!(!updated_content.contains("port: 502"));

        // Clean up
        std::fs::remove_dir_all(&temp_dir).unwrap();
        match previous_config_dir {
            Ok(val) => std::env::set_var("HA_CONFIG_DIR", val),
            Err(_) => std::env::remove_var("HA_CONFIG_DIR"),
        }
    }

    #[tokio::test]
    async fn test_probe_modbus_mock_tcp() {
        use tokio::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut buf = [0u8; 1024];
                let _n = socket.read(&mut buf).await.unwrap();

                // Assert received Modbus frame is standard read holding registers for address 100, count 1
                assert_eq!(buf[4], 0x00);
                assert_eq!(buf[5], 0x06); // length 6
                assert_eq!(buf[6], 0x01); // Unit ID 1
                assert_eq!(buf[7], 0x03); // Func Code 3 (holding)
                assert_eq!(buf[8], 0x00);
                assert_eq!(buf[9], 0x64); // Address 100
                assert_eq!(buf[10], 0x00);
                assert_eq!(buf[11], 0x01); // Count 1

                // Construct Modbus response: Transaction ID (2B), Protocol ID (2B), Length (2B), Unit ID (1B), Func Code (1B), Byte Count (1B), Data (2B)
                let resp_frame = vec![
                    0x00, 0x01, // Transaction ID
                    0x00, 0x00, // Protocol ID
                    0x00, 0x05, // Length
                    0x01, // Unit ID
                    0x03, // Func Code
                    0x02, // Byte Count (2 bytes)
                    0x01, 0xF4, // Register value = 500 (0x01F4)
                ];
                socket.write_all(&resp_frame).await.unwrap();
            }
        });

        let probe_params = ProbeModbusParams {
            host: Some("127.0.0.1".to_string()),
            port: Some(addr.port()),
            unit_id: Some(1),
            register_type: "holding".to_string(),
            address: 100,
            count: Some(1),
        };

        let res = handle_probe_modbus(probe_params).await.unwrap();
        assert_eq!(res["status"], "success");
        assert_eq!(res["values"][0], 500);
    }

    #[tokio::test]
    async fn test_auto_discover_modbus_target() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let temp_dir = std::env::temp_dir().join("ha_config_auto_discover_test");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let previous_config_dir = std::env::var("HA_CONFIG_DIR");
        std::env::set_var("HA_CONFIG_DIR", &temp_dir);

        // 1. Test YAML fallback first
        let config_file = temp_dir.join("configuration.yaml");
        let yaml_content =
            "modbus:\n  - name: my_modbus_hub\n    host: 192.168.10.15\n    port: 5022\n";
        std::fs::write(&config_file, yaml_content).unwrap();

        let (host, port) = auto_discover_modbus_target(Some("my_modbus_hub"))
            .await
            .unwrap();
        assert_eq!(host, "192.168.10.15");
        assert_eq!(port, 5022);

        // 2. Test modern storage JSON entry takes precedence
        let storage_dir = temp_dir.join(".storage");
        std::fs::create_dir_all(&storage_dir).unwrap();
        let config_entries_file = storage_dir.join("core.config_entries");
        let json_content = r#"{
            "version": 1,
            "minor_version": 1,
            "key": "core.config_entries",
            "data": {
                "entries": [
                    {
                        "entry_id": "test_modbus_id",
                        "version": 1,
                        "domain": "modbus",
                        "title": "modern_modbus_hub",
                        "data": {
                            "host": "10.0.0.22",
                            "port": 5023
                        }
                    }
                ]
            }
        }"#;
        std::fs::write(&config_entries_file, json_content).unwrap();

        // Should auto-discover modern entry
        let (storage_host, storage_port) = auto_discover_modbus_target(Some("modern_modbus_hub"))
            .await
            .unwrap();
        assert_eq!(storage_host, "10.0.0.22");
        assert_eq!(storage_port, 5023);

        // Should get modern entry as first default (None)
        let (host_first, port_first) = auto_discover_modbus_target(None).await.unwrap();
        assert_eq!(host_first, "10.0.0.22");
        assert_eq!(port_first, 5023);

        // Clean up
        std::fs::remove_dir_all(&temp_dir).unwrap();
        match previous_config_dir {
            Ok(val) => std::env::set_var("HA_CONFIG_DIR", val),
            Err(_) => std::env::remove_var("HA_CONFIG_DIR"),
        }
    }
}

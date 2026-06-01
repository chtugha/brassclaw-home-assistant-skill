use crate::helpers::{
    compact_entity, days_to_ymd, normalize_url, truncate_template_output, url_encode,
    validate_domain, validate_entity_id, validate_event_type, validate_ha_url, validate_iso_prefix,
    validate_not_empty, validate_service, MAX_ENTITY_ID_LEN, MAX_MQTT_TOPIC_LEN, MAX_STATE_LEN,
    MAX_TEMPLATE_LEN,
};
use crate::near::agent::host;
use crate::types::StatesResponse;

const MAX_STATES: usize = 5000;
const MAX_HOURS_BACK: u32 = 8760;
const MAX_TEMPLATE_OUT_BYTES: u32 = 16_384;
const DEFAULT_TEMPLATE_OUT_BYTES: u32 = 8_192;
const MS_PER_SECOND: u64 = 1000;
const SECONDS_PER_MINUTE: u64 = 60;
const SECONDS_PER_HOUR: u64 = 3600;
const SECONDS_PER_DAY: u64 = 86400;

fn iso_timestamp_hours_ago(hours_back: u32) -> String {
    let now_ms = host::now_millis();
    let start_ms = now_ms.saturating_sub((hours_back as u64) * SECONDS_PER_HOUR * MS_PER_SECOND);
    let secs = start_ms / MS_PER_SECOND;
    let d = secs / SECONDS_PER_DAY;
    let rem = secs % SECONDS_PER_DAY;
    let h = rem / SECONDS_PER_HOUR;
    let m = (rem % SECONDS_PER_HOUR) / SECONDS_PER_MINUTE;
    let s = rem % SECONDS_PER_MINUTE;
    let (y, mo, day) = days_to_ymd(d as i64);
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", y, mo, day, h, m, s)
}

fn ha_get(base: &str, path: &str) -> Result<String, String> {
    validate_ha_url(base)?;
    let url = format!("{}{}", normalize_url(base), path);
    host::log(host::LogLevel::Debug, &format!("GET {}", path));
    let resp = host::http_request("GET", &url, "{}", None, None)?;
    if resp.status < 200 || resp.status >= 300 {
        return Err(format!(
            "HA API {} returned {}: {}",
            path,
            resp.status,
            String::from_utf8_lossy(&resp.body)
        ));
    }
    String::from_utf8(resp.body).map_err(|e| format!("Invalid UTF-8: {}", e))
}

fn ha_post(base: &str, path: &str, body: Option<&str>) -> Result<String, String> {
    validate_ha_url(base)?;
    let url = format!("{}{}", normalize_url(base), path);
    let body_str = body.unwrap_or("{}");
    let body_bytes = body_str.as_bytes().to_vec();
    host::log(host::LogLevel::Debug, &format!("POST {}", path));
    let resp = host::http_request(
        "POST",
        &url,
        r#"{"Content-Type": "application/json"}"#,
        Some(&body_bytes),
        None,
    )?;
    if resp.status < 200 || resp.status >= 300 {
        return Err(format!(
            "HA API {} returned {}: {}",
            path,
            resp.status,
            String::from_utf8_lossy(&resp.body)
        ));
    }
    String::from_utf8(resp.body).map_err(|e| format!("Invalid UTF-8: {}", e))
}

pub fn get_status(base: &str) -> Result<String, String> {
    ha_get(base, "/api/")
}

pub fn get_config(base: &str) -> Result<String, String> {
    ha_get(base, "/api/config")
}

pub fn get_states(
    base: &str,
    domain_filter: Option<&[&str]>,
    max_items: Option<u32>,
    compact: bool,
) -> Result<String, String> {
    if let Some(domains) = domain_filter {
        for d in domains {
            validate_domain(d)?;
        }
    }
    let raw = ha_get(base, "/api/states")?;
    let all: Vec<serde_json::Value> =
        serde_json::from_str(&raw).map_err(|e| format!("Failed to parse states: {}", e))?;
    let total_unfiltered = all.len();

    let filtered: Vec<serde_json::Value> = if let Some(domains) = domain_filter {
        let prefixes: Vec<String> = domains.iter().map(|d| format!("{}.", d)).collect();
        all.into_iter()
            .filter(|e| {
                e.get("entity_id")
                    .and_then(|v| v.as_str())
                    .map(|id| prefixes.iter().any(|p| id.starts_with(p.as_str())))
                    .unwrap_or(false)
            })
            .collect()
    } else {
        all
    };

    let matched = filtered.len();
    let (cap, user_cap) = match max_items {
        Some(0) => return Err("max_items must be >= 1".into()),
        Some(n) => {
            let user = n as usize;
            (user.min(MAX_STATES), Some(user))
        }
        None => (MAX_STATES, None),
    };
    let (mut entities, truncated, cap_kind) = if matched > cap {
        let kind = match user_cap {
            Some(u) if u <= MAX_STATES => "user",
            _ => "hard",
        };
        (filtered[..cap].to_vec(), Some(true), Some(kind))
    } else {
        (filtered, None, None)
    };

    if compact {
        for e in entities.iter_mut() {
            *e = compact_entity(e);
        }
    }

    let resp = StatesResponse {
        count: entities.len(),
        matched,
        total: total_unfiltered,
        entities,
        truncated,
        cap_kind,
    };
    serde_json::to_string(&resp).map_err(|e| e.to_string())
}

pub fn get_state(base: &str, entity_id: &str) -> Result<String, String> {
    validate_entity_id(entity_id)?;
    ha_get(base, &format!("/api/states/{}", url_encode(entity_id)))
}

pub fn set_state(
    base: &str,
    entity_id: &str,
    state: &str,
    attributes: Option<&serde_json::Value>,
) -> Result<String, String> {
    validate_entity_id(entity_id)?;
    validate_not_empty(state, "state")?;
    if state.len() > MAX_STATE_LEN {
        return Err(format!(
            "state value too long (max {} characters)",
            MAX_STATE_LEN
        ));
    }
    let mut body = serde_json::json!({"state": state});
    if let Some(attrs) = attributes {
        if !attrs.is_object() {
            return Err(
                "attributes must be a JSON object (e.g. {\"unit_of_measurement\": \"°C\"})".into(),
            );
        }
        body["attributes"] = attrs.clone();
    }
    let body_str = serde_json::to_string(&body).map_err(|e| e.to_string())?;
    ha_post(
        base,
        &format!("/api/states/{}", url_encode(entity_id)),
        Some(&body_str),
    )
}

pub fn delete_state(base: &str, entity_id: &str) -> Result<String, String> {
    validate_entity_id(entity_id)?;
    validate_ha_url(base)?;
    let url = format!(
        "{}/api/states/{}",
        normalize_url(base),
        url_encode(entity_id)
    );
    host::log(
        host::LogLevel::Debug,
        &format!("DELETE /api/states/{}", entity_id),
    );
    let resp = host::http_request("DELETE", &url, "{}", None, None)?;
    if resp.status < 200 || resp.status >= 300 {
        return Err(format!(
            "HA API DELETE /api/states/{} returned {}: {}",
            entity_id,
            resp.status,
            String::from_utf8_lossy(&resp.body)
        ));
    }
    let body = String::from_utf8(resp.body).map_err(|e| format!("Invalid UTF-8: {}", e))?;
    if body.trim().is_empty() {
        Ok(serde_json::json!({"deleted": entity_id}).to_string())
    } else {
        Ok(body)
    }
}

pub fn call_service(
    base: &str,
    domain: &str,
    service: &str,
    data: Option<&serde_json::Value>,
) -> Result<String, String> {
    validate_domain(domain)?;
    validate_service(service)?;
    let path = format!(
        "/api/services/{}/{}",
        url_encode(domain),
        url_encode(service)
    );
    let body_str = match data {
        Some(d) => serde_json::to_string(d).expect("serializing a serde_json::Value is infallible"),
        None => "{}".to_string(),
    };
    ha_post(base, &path, Some(&body_str))
}

pub fn get_services(base: &str) -> Result<String, String> {
    ha_get(base, "/api/services")
}

pub fn fire_event(
    base: &str,
    event_type: &str,
    event_data: Option<&serde_json::Value>,
) -> Result<String, String> {
    validate_event_type(event_type)?;
    let path = format!("/api/events/{}", url_encode(event_type));
    let body_str = match event_data {
        Some(d) => serde_json::to_string(d).expect("serializing a serde_json::Value is infallible"),
        None => "{}".to_string(),
    };
    ha_post(base, &path, Some(&body_str))
}

pub fn render_template(
    base: &str,
    template: &str,
    variables: Option<&serde_json::Value>,
    max_chars: Option<u32>,
) -> Result<String, String> {
    validate_not_empty(template, "template")?;
    if template.len() > MAX_TEMPLATE_LEN {
        return Err(format!(
            "template too large (max {} bytes)",
            MAX_TEMPLATE_LEN
        ));
    }
    let mut body = serde_json::json!({"template": template});
    if let Some(v) = variables {
        if !v.is_object() {
            return Err("variables must be a JSON object".into());
        }
        body["variables"] = v.clone();
    }
    let body_str = serde_json::to_string(&body).map_err(|e| e.to_string())?;
    let raw = ha_post(base, "/api/template", Some(&body_str))?;
    let cap = max_chars
        .unwrap_or(DEFAULT_TEMPLATE_OUT_BYTES)
        .min(MAX_TEMPLATE_OUT_BYTES) as usize;
    Ok(truncate_template_output(raw, cap))
}

pub fn get_history(
    base: &str,
    entity_id: &str,
    hours_back: u32,
    start_time: Option<&str>,
    end_time: Option<&str>,
) -> Result<String, String> {
    validate_entity_id(entity_id)?;
    if start_time.is_none() && (hours_back == 0 || hours_back > MAX_HOURS_BACK) {
        return Err(format!(
            "hours_back must be between 1 and {}",
            MAX_HOURS_BACK
        ));
    }
    let ts = if let Some(st) = start_time {
        validate_iso_prefix(st, "start_time")?;
        st.to_string()
    } else {
        iso_timestamp_hours_ago(hours_back)
    };
    let mut path = format!(
        "/api/history/period/{}?filter_entity_id={}",
        url_encode(&ts),
        url_encode(entity_id)
    );
    if let Some(et) = end_time {
        validate_iso_prefix(et, "end_time")?;
        path.push_str(&format!("&end_time={}", url_encode(et)));
    }
    ha_get(base, &path)
}

pub fn get_logbook(
    base: &str,
    entity_id: Option<&str>,
    hours_back: u32,
    start_time: Option<&str>,
    end_time: Option<&str>,
) -> Result<String, String> {
    if let Some(eid) = entity_id {
        validate_entity_id(eid)?;
    }
    let ts = if let Some(st) = start_time {
        validate_iso_prefix(st, "start_time")?;
        st.to_string()
    } else {
        if hours_back == 0 || hours_back > MAX_HOURS_BACK {
            return Err(format!(
                "hours_back must be between 1 and {}",
                MAX_HOURS_BACK
            ));
        }
        iso_timestamp_hours_ago(hours_back)
    };
    let mut path = format!("/api/logbook/{}", url_encode(&ts));
    let mut has_query = false;
    if let Some(eid) = entity_id {
        path.push_str(&format!("?entity={}", url_encode(eid)));
        has_query = true;
    }
    if let Some(et) = end_time {
        validate_iso_prefix(et, "end_time")?;
        let sep = if has_query { '&' } else { '?' };
        path.push_str(&format!("{}end_time={}", sep, url_encode(et)));
    }
    ha_get(base, &path)
}

pub fn get_calendar_events(
    base: &str,
    entity_id: &str,
    start: &str,
    end: &str,
) -> Result<String, String> {
    validate_entity_id(entity_id)?;
    validate_iso_prefix(start, "start")?;
    validate_iso_prefix(end, "end")?;
    let path = format!(
        "/api/calendars/{}?start={}&end={}",
        url_encode(entity_id),
        url_encode(start),
        url_encode(end)
    );
    ha_get(base, &path)
}

pub fn toggle_automation(base: &str, entity_id: &str, enabled: bool) -> Result<String, String> {
    validate_entity_id(entity_id)?;
    if !entity_id.starts_with("automation.") {
        return Err(format!(
            "entity_id '{}' must start with 'automation.'",
            entity_id
        ));
    }
    let service = if enabled { "turn_on" } else { "turn_off" };
    call_service(
        base,
        "automation",
        service,
        Some(&serde_json::json!({"entity_id": entity_id})),
    )
}

pub fn trigger_automation(base: &str, entity_id: &str) -> Result<String, String> {
    validate_entity_id(entity_id)?;
    if !entity_id.starts_with("automation.") {
        return Err(format!(
            "entity_id '{}' must start with 'automation.'",
            entity_id
        ));
    }
    call_service(
        base,
        "automation",
        "trigger",
        Some(&serde_json::json!({"entity_id": entity_id})),
    )
}

pub fn run_script(
    base: &str,
    entity_id: &str,
    variables: Option<&serde_json::Value>,
) -> Result<String, String> {
    validate_entity_id(entity_id)?;
    if !entity_id.starts_with("script.") {
        return Err(format!(
            "entity_id '{}' must start with 'script.'",
            entity_id
        ));
    }
    let script_id = entity_id.strip_prefix("script.").unwrap_or(entity_id);
    let data = variables.cloned().unwrap_or(serde_json::json!({}));
    call_service(base, "script", script_id, Some(&data))
}

pub fn activate_scene(base: &str, entity_id: &str) -> Result<String, String> {
    validate_entity_id(entity_id)?;
    if !entity_id.starts_with("scene.") {
        return Err(format!(
            "entity_id '{}' must start with 'scene.'",
            entity_id
        ));
    }
    call_service(
        base,
        "scene",
        "turn_on",
        Some(&serde_json::json!({"entity_id": entity_id})),
    )
}

pub fn mqtt_publish(
    base: &str,
    topic: &str,
    payload: &str,
    qos: Option<u8>,
    retain: Option<bool>,
) -> Result<String, String> {
    validate_not_empty(topic, "topic")?;
    if topic.len() > MAX_MQTT_TOPIC_LEN {
        return Err(format!(
            "topic too long (max {} bytes per MQTT spec)",
            MAX_MQTT_TOPIC_LEN
        ));
    }
    if topic.contains('\0') {
        return Err("topic must not contain null characters".into());
    }
    let mut data = serde_json::json!({"topic": topic, "payload": payload});
    if let Some(q) = qos {
        if q > 2 {
            return Err("qos must be 0, 1, or 2".into());
        }
        data["qos"] = serde_json::json!(q);
    }
    if let Some(r) = retain {
        data["retain"] = serde_json::json!(r);
    }
    call_service(base, "mqtt", "publish", Some(&data))
}

pub fn modbus_write(
    base: &str,
    hub: Option<&str>,
    unit: u16,
    address: u16,
    value: &serde_json::Value,
    write_type: &str,
) -> Result<String, String> {
    let mut svc_data = serde_json::json!({"unit": unit, "address": address, "value": value});
    if let Some(h) = hub {
        svc_data["hub"] = serde_json::json!(h);
    }
    match write_type {
        "coil" => {
            if value.is_boolean() {
            } else if let Some(arr) = value.as_array() {
                if arr.is_empty() {
                    return Err("value array must not be empty for coil writes".into());
                }
                if !arr.iter().all(|v| v.is_boolean()) {
                    return Err("value array elements must all be booleans for coil writes".into());
                }
            } else {
                return Err("value must be a boolean or array of booleans for coil writes".into());
            }
            call_service(base, "modbus", "write_coil", Some(&svc_data))
        }
        "holding" => {
            if value.is_number() {
            } else if let Some(arr) = value.as_array() {
                if arr.is_empty() {
                    return Err("value array must not be empty for holding register writes".into());
                }
                if !arr.iter().all(|v| v.is_number()) {
                    return Err(
                        "value array elements must all be numbers for holding register writes"
                            .into(),
                    );
                }
            } else {
                return Err(
                    "value must be a number or array of numbers for holding register writes".into(),
                );
            }
            call_service(base, "modbus", "write_register", Some(&svc_data))
        }
        _ => Err(format!(
            "write_type must be 'coil' or 'holding', got '{}'",
            write_type
        )),
    }
}

pub fn get_notifications(base: &str) -> Result<String, String> {
    ha_get(base, "/api/persistent_notification")
}

pub fn dismiss_notification(base: &str, notification_id: &str) -> Result<String, String> {
    validate_not_empty(notification_id, "notification_id")?;
    if notification_id.len() > MAX_ENTITY_ID_LEN {
        return Err("notification_id too long".into());
    }
    call_service(
        base,
        "persistent_notification",
        "dismiss",
        Some(&serde_json::json!({"notification_id": notification_id})),
    )
}

pub fn check_config(base: &str) -> Result<String, String> {
    ha_post(base, "/api/config/core/check_config", Some("{}"))
}

pub fn get_error_log(base: &str, tail_lines: Option<u32>) -> Result<String, String> {
    if let Some(n) = tail_lines {
        if n == 0 {
            return Err("tail_lines must be >= 1".into());
        }
    }
    let full = ha_get(base, "/api/error_log")?;
    if let Some(n) = tail_lines {
        let n = n as usize;
        let lines: Vec<&str> = full.lines().collect();
        let start = lines.len().saturating_sub(n);
        return Ok(lines[start..].join("\n"));
    }
    Ok(full)
}

pub fn restart_ha(base: &str) -> Result<String, String> {
    call_service(base, "homeassistant", "restart", None)
}

pub fn reload_core_config(base: &str) -> Result<String, String> {
    call_service(base, "homeassistant", "reload_core_config", None)
}

pub fn reload_automations(base: &str) -> Result<String, String> {
    call_service(base, "automation", "reload", None)
}

pub fn reload_scripts(base: &str) -> Result<String, String> {
    call_service(base, "script", "reload", None)
}

pub fn reload_scenes(base: &str) -> Result<String, String> {
    call_service(base, "scene", "reload", None)
}

pub fn reload_themes(base: &str) -> Result<String, String> {
    call_service(base, "frontend", "reload_themes", None)
}

pub fn get_config_entries(base: &str, domain: Option<&str>) -> Result<String, String> {
    if let Some(d) = domain {
        validate_domain(d)?;
    }
    let path = match domain {
        Some(d) => format!("/api/config/config_entries/entry?domain={}", url_encode(d)),
        None => "/api/config/config_entries/entry".to_string(),
    };
    ha_get(base, &path)
}

pub fn reload_config_entry(base: &str, entry_id: &str) -> Result<String, String> {
    validate_not_empty(entry_id, "entry_id")?;
    if entry_id.len() > MAX_ENTITY_ID_LEN {
        return Err("entry_id too long".into());
    }
    for c in entry_id.chars() {
        if !c.is_alphanumeric() && c != '_' && c != '-' {
            return Err(format!("entry_id contains invalid character '{}'", c));
        }
    }
    call_service(
        base,
        "homeassistant",
        "reload_config_entry",
        Some(&serde_json::json!({"entry_id": entry_id})),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reload_config_entry_validation() {
        let base = "http://192.168.1.1:8123";
        assert!(reload_config_entry(base, "")
            .unwrap_err()
            .contains("must not be empty"));
        assert!(reload_config_entry(base, "bad/id")
            .unwrap_err()
            .contains("invalid character"));
        assert!(reload_config_entry(base, "bad id")
            .unwrap_err()
            .contains("invalid character"));
        assert!(
            reload_config_entry(base, &"a".repeat(MAX_ENTITY_ID_LEN + 1))
                .unwrap_err()
                .contains("too long")
        );
    }

    #[test]
    fn test_modbus_write_value_validation() {
        let base = "http://192.168.1.1:8123";
        assert!(modbus_write(base, None, 1, 0, &serde_json::json!("string"), "coil").is_err());
        assert!(modbus_write(base, None, 1, 0, &serde_json::json!(42), "coil").is_err());
        assert!(modbus_write(base, None, 1, 0, &serde_json::json!([true, 42]), "coil").is_err());
        assert!(modbus_write(base, None, 1, 0, &serde_json::json!([]), "coil").is_err());

        assert!(modbus_write(base, None, 1, 0, &serde_json::json!("string"), "holding").is_err());
        assert!(modbus_write(base, None, 1, 0, &serde_json::json!(true), "holding").is_err());
        assert!(modbus_write(base, None, 1, 0, &serde_json::json!([1, "two"]), "holding").is_err());
        assert!(modbus_write(base, None, 1, 0, &serde_json::json!([]), "holding").is_err());

        assert!(modbus_write(base, None, 1, 0, &serde_json::json!(42), "invalid").is_err());
    }

    #[test]
    fn test_get_config_entries_domain_validation() {
        let base = "http://192.168.1.1:8123";
        assert!(get_config_entries(base, Some("bad.domain")).is_err());
        assert!(get_config_entries(base, Some("bad domain")).is_err());
        assert!(get_config_entries(base, Some("")).is_err());
    }
}

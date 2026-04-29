use crate::near::agent::host;
use crate::shell::{self, SshConfig};
use crate::types::StatesResponse;

const MAX_STATES: usize = 5000;
const MAX_HOURS_BACK: u32 = 8760;
const MAX_ENTITY_ID_LEN: usize = 255;
const MAX_EVENT_TYPE_LEN: usize = 255;
const MAX_STATE_LEN: usize = 255;
const MAX_TEMPLATE_LEN: usize = 65_536;
const MAX_TEMPLATE_OUT_BYTES: u32 = 16_384;
const DEFAULT_TEMPLATE_OUT_BYTES: u32 = 8_192;
const MAX_MQTT_TOPIC_LEN: usize = 65_535;
const DEFAULT_HA_LOG_PATH: &str = "/config/home-assistant.log";
const DEFAULT_SHELL_TAIL_LINES: u32 = 200;
const MS_PER_SECOND: u64 = 1000;
const SECONDS_PER_MINUTE: u64 = 60;
const SECONDS_PER_HOUR: u64 = 3600;
const SECONDS_PER_DAY: u64 = 86400;

fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => {
                out.push('%');
                out.push(char::from(b"0123456789ABCDEF"[(b >> 4) as usize]));
                out.push(char::from(b"0123456789ABCDEF"[(b & 0xf) as usize]));
            }
        }
    }
    out
}

fn validate_ha_url(ha_url: &str) -> Result<(), String> {
    let lower = ha_url.to_lowercase();
    if !lower.starts_with("http://") && !lower.starts_with("https://") {
        return Err("ha_url must start with http:// or https://".into());
    }
    let host_part = if let Some(s) = lower.strip_prefix("http://") {
        s
    } else {
        lower.strip_prefix("https://").unwrap_or("")
    };
    let host = host_part.split('/').next().unwrap_or("");
    // Defense-in-depth: explicitly reject userinfo, query, or fragment
    // characters appearing inside the authority component.
    if host.contains('@') || host.contains('?') || host.contains('#') {
        return Err(
            "ha_url authority must not contain '@', '?', or '#' (no userinfo/query/fragment in host)"
                .into(),
        );
    }
    let host_no_port = host.split(':').next().unwrap_or("");
    if host_no_port.is_empty() {
        return Err("ha_url must contain a hostname".into());
    }
    let is_private = host_no_port == "localhost"
        || host_no_port == "127.0.0.1"
        || is_private_ip(host_no_port, "192.168.")
        || is_private_ip(host_no_port, "10.")
        || is_private_172(host_no_port)
        || host_no_port.ends_with(".local")
        || host_no_port.ends_with(".internal")
        || host_no_port.ends_with(".lan")
        || host_no_port.ends_with(".home")
        || host_no_port.ends_with(".duckdns.org")
        || host_no_port.ends_with(".nabu.casa");
    if !is_private {
        return Err(format!(
            "ha_url host '{}' is not a recognized private/local address. \
             Allowed: localhost, 127.0.0.1, 192.168.*, 10.*, 172.16-31.*, \
             *.local, *.internal, *.lan, *.home, *.duckdns.org, *.nabu.casa",
            host_no_port
        ));
    }
    Ok(())
}

// Structural IPv4 shape check used only to guard the private-address SSRF
// check. Octets are NOT range-clamped to 0-255 because the prefix match
// (192.168., 10., 172.16-31.) already constrains the meaningful octets,
// and accepting "999" in a trailing octet cannot escape a private prefix.
fn is_ip_only(s: &str) -> bool {
    if s.is_empty() || s.starts_with('.') || s.ends_with('.') || s.contains("..") {
        return false;
    }
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 4 {
        return false;
    }
    parts.iter().all(|p| !p.is_empty() && p.len() <= 3 && p.bytes().all(|b| b.is_ascii_digit()))
}

fn is_private_ip(host: &str, prefix: &str) -> bool {
    host.starts_with(prefix) && is_ip_only(host)
}

fn is_private_172(host: &str) -> bool {
    if let Some(rest) = host.strip_prefix("172.") {
        if !is_ip_only(host) {
            return false;
        }
        if let Some(second) = rest.split('.').next() {
            if let Ok(n) = second.parse::<u8>() {
                return (16..=31).contains(&n);
            }
        }
    }
    false
}

/// Normalize an HA base URL: lowercase the scheme + authority (host[:port])
/// while preserving case-sensitive path bytes, and trim a trailing `/`.
/// Assumes `validate_ha_url` already accepted the input.
fn normalize_url(ha_url: &str) -> String {
    let trimmed = ha_url.trim_end_matches('/');
    let (scheme_len, scheme) = if trimmed.len() >= 7
        && trimmed[..7].eq_ignore_ascii_case("http://")
    {
        (7, "http://")
    } else if trimmed.len() >= 8 && trimmed[..8].eq_ignore_ascii_case("https://") {
        (8, "https://")
    } else {
        return trimmed.to_string();
    };
    let after_scheme = &trimmed[scheme_len..];
    let (authority, path) = match after_scheme.find('/') {
        Some(i) => (&after_scheme[..i], &after_scheme[i..]),
        None => (after_scheme, ""),
    };
    let mut out = String::with_capacity(trimmed.len());
    out.push_str(scheme);
    out.push_str(&authority.to_ascii_lowercase());
    out.push_str(path);
    out
}

fn validate_entity_id(id: &str) -> Result<(), String> {
    if id.is_empty() {
        return Err("entity_id must not be empty".into());
    }
    if !id.contains('.') {
        return Err(format!("entity_id '{}' must contain a dot (e.g. 'light.living_room')", id));
    }
    if id.len() > MAX_ENTITY_ID_LEN {
        return Err("entity_id too long".into());
    }
    for c in id.chars() {
        if !c.is_alphanumeric() && c != '.' && c != '_' && c != '-' {
            return Err(format!("entity_id contains invalid character '{}'", c));
        }
    }
    Ok(())
}

fn validate_domain(d: &str) -> Result<(), String> {
    if d.is_empty() {
        return Err("domain must not be empty".into());
    }
    for c in d.chars() {
        if !c.is_alphanumeric() && c != '_' {
            return Err(format!("domain contains invalid character '{}'", c));
        }
    }
    Ok(())
}

fn validate_service(s: &str) -> Result<(), String> {
    if s.is_empty() {
        return Err("service must not be empty".into());
    }
    for c in s.chars() {
        if !c.is_alphanumeric() && c != '_' && c != '-' {
            return Err(format!("service contains invalid character '{}'", c));
        }
    }
    Ok(())
}

fn validate_iso_prefix(s: &str, field: &str) -> Result<(), String> {
    let b = s.as_bytes();
    if b.len() < 11
        || !b[0..4].iter().all(|c| c.is_ascii_digit())
        || b[4] != b'-'
        || !b[5..7].iter().all(|c| c.is_ascii_digit())
        || b[7] != b'-'
        || !b[8..10].iter().all(|c| c.is_ascii_digit())
        || b[10] != b'T'
    {
        return Err(format!("{} must be ISO 8601 format (YYYY-MM-DDThh:mm:ss)", field));
    }
    let month = (b[5] - b'0') * 10 + (b[6] - b'0');
    let day = (b[8] - b'0') * 10 + (b[9] - b'0');
    if !(1..=12).contains(&month) {
        return Err(format!("{} month component must be 01-12", field));
    }
    if !(1..=31).contains(&day) {
        return Err(format!("{} day component must be 01-31", field));
    }
    Ok(())
}

/// Format `now - hours_back` as an ISO 8601 UTC timestamp (`YYYY-MM-DDThh:mm:ssZ`).
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
        return Err(format!("HA API {} returned {}: {}", path, resp.status, String::from_utf8_lossy(&resp.body)));
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
        return Err(format!("HA API {} returned {}: {}", path, resp.status, String::from_utf8_lossy(&resp.body)));
    }
    String::from_utf8(resp.body).map_err(|e| format!("Invalid UTF-8: {}", e))
}

pub fn get_status(base: &str) -> Result<String, String> {
    ha_get(base, "/api/")
}

pub fn get_config(base: &str) -> Result<String, String> {
    ha_get(base, "/api/config")
}

/// Project a full HA state object to its discovery-relevant subset:
/// `{entity_id, state, last_changed?}`. Drops the verbose `attributes`
/// map and timestamps the agent rarely needs during enumeration.
fn compact_entity(e: &serde_json::Value) -> serde_json::Value {
    let entity_id = e.get("entity_id").cloned().unwrap_or(serde_json::Value::Null);
    let state = e.get("state").cloned().unwrap_or(serde_json::Value::Null);
    let last_changed = e.get("last_changed").cloned();
    let mut obj = serde_json::Map::new();
    obj.insert("entity_id".into(), entity_id);
    obj.insert("state".into(), state);
    if let Some(lc) = last_changed {
        obj.insert("last_changed".into(), lc);
    }
    serde_json::Value::Object(obj)
}

/// Cap a rendered-template body at `cap` UTF-8 bytes, appending a
/// truncation marker that tells the agent how many bytes were elided
/// and how to widen the cap. Returns `raw` unchanged when within cap.
fn truncate_template_output(raw: String, cap: usize) -> String {
    if raw.len() <= cap {
        return raw;
    }
    let mut end = cap;
    while end > 0 && !raw.is_char_boundary(end) {
        end -= 1;
    }
    let mut out = String::with_capacity(end + 64);
    out.push_str(&raw[..end]);
    out.push_str(&format!(
        "\n…[truncated, {} more bytes — pass `max_chars` to widen]",
        raw.len() - end
    ));
    out
}

pub fn get_states(
    base: &str,
    domain_filter: Option<&[&str]>,
    max_items: Option<u32>,
    compact: bool,
) -> Result<String, String> {
    // Validate domain filter cheaply *before* the network round-trip so
    // garbage input fails fast without fetching every HA entity.
    if let Some(domains) = domain_filter {
        for d in domains {
            validate_domain(d)?;
        }
    }
    let raw = ha_get(base, "/api/states")?;
    let all: Vec<serde_json::Value> = serde_json::from_str(&raw)
        .map_err(|e| format!("Failed to parse states: {}", e))?;
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

pub fn set_state(base: &str, entity_id: &str, state: &str, attributes: Option<&serde_json::Value>) -> Result<String, String> {
    validate_entity_id(entity_id)?;
    validate_not_empty(state, "state")?;
    if state.len() > MAX_STATE_LEN {
        return Err(format!("state value too long (max {} characters)", MAX_STATE_LEN));
    }
    let mut body = serde_json::json!({"state": state});
    if let Some(attrs) = attributes {
        if !attrs.is_object() {
            return Err("attributes must be a JSON object (e.g. {\"unit_of_measurement\": \"°C\"})".into());
        }
        body["attributes"] = attrs.clone();
    }
    let body_str = serde_json::to_string(&body).map_err(|e| e.to_string())?;
    ha_post(base, &format!("/api/states/{}", url_encode(entity_id)), Some(&body_str))
}

pub fn delete_state(base: &str, entity_id: &str) -> Result<String, String> {
    validate_entity_id(entity_id)?;
    validate_ha_url(base)?;
    let url = format!("{}/api/states/{}", normalize_url(base), url_encode(entity_id));
    host::log(host::LogLevel::Debug, &format!("DELETE /api/states/{}", entity_id));
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

pub fn call_service(base: &str, domain: &str, service: &str, data: Option<&serde_json::Value>) -> Result<String, String> {
    validate_domain(domain)?;
    validate_service(service)?;
    let path = format!("/api/services/{}/{}", url_encode(domain), url_encode(service));
    let body_str = match data {
        Some(d) => serde_json::to_string(d).expect("serializing a serde_json::Value is infallible"),
        None => "{}".to_string(),
    };
    ha_post(base, &path, Some(&body_str))
}

pub fn get_services(base: &str) -> Result<String, String> {
    ha_get(base, "/api/services")
}

fn validate_event_type(s: &str) -> Result<(), String> {
    if s.is_empty() || s.len() > MAX_EVENT_TYPE_LEN {
        return Err(format!("event_type must be 1-{} characters", MAX_EVENT_TYPE_LEN));
    }
    for c in s.chars() {
        if !c.is_alphanumeric() && c != '_' && c != '.' && c != '-' {
            return Err(format!("event_type contains invalid character '{}'", c));
        }
    }
    Ok(())
}

fn validate_not_empty(value: &str, field: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err(format!("{} must not be empty", field));
    }
    Ok(())
}

pub fn fire_event(base: &str, event_type: &str, event_data: Option<&serde_json::Value>) -> Result<String, String> {
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
        return Err(format!("template too large (max {} bytes)", MAX_TEMPLATE_LEN));
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
        return Err(format!("hours_back must be between 1 and {}", MAX_HOURS_BACK));
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
            return Err(format!("hours_back must be between 1 and {}", MAX_HOURS_BACK));
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

pub fn get_calendar_events(base: &str, entity_id: &str, start: &str, end: &str) -> Result<String, String> {
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
        return Err(format!("entity_id '{}' must start with 'automation.'", entity_id));
    }
    let service = if enabled { "turn_on" } else { "turn_off" };
    call_service(base, "automation", service, Some(&serde_json::json!({"entity_id": entity_id})))
}

pub fn trigger_automation(base: &str, entity_id: &str) -> Result<String, String> {
    validate_entity_id(entity_id)?;
    if !entity_id.starts_with("automation.") {
        return Err(format!("entity_id '{}' must start with 'automation.'", entity_id));
    }
    call_service(base, "automation", "trigger", Some(&serde_json::json!({"entity_id": entity_id})))
}

pub fn run_script(base: &str, entity_id: &str, variables: Option<&serde_json::Value>) -> Result<String, String> {
    validate_entity_id(entity_id)?;
    if !entity_id.starts_with("script.") {
        return Err(format!("entity_id '{}' must start with 'script.'", entity_id));
    }
    let script_id = entity_id.strip_prefix("script.").unwrap_or(entity_id);
    let data = variables.cloned().unwrap_or(serde_json::json!({}));
    call_service(base, "script", script_id, Some(&data))
}

pub fn activate_scene(base: &str, entity_id: &str) -> Result<String, String> {
    validate_entity_id(entity_id)?;
    if !entity_id.starts_with("scene.") {
        return Err(format!("entity_id '{}' must start with 'scene.'", entity_id));
    }
    call_service(base, "scene", "turn_on", Some(&serde_json::json!({"entity_id": entity_id})))
}

pub fn mqtt_publish(base: &str, topic: &str, payload: &str, qos: Option<u8>, retain: Option<bool>) -> Result<String, String> {
    validate_not_empty(topic, "topic")?;
    if topic.len() > MAX_MQTT_TOPIC_LEN {
        return Err(format!("topic too long (max {} bytes per MQTT spec)", MAX_MQTT_TOPIC_LEN));
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

pub fn modbus_write(base: &str, hub: Option<&str>, unit: u16, address: u16, value: &serde_json::Value, write_type: &str) -> Result<String, String> {
    let mut svc_data = serde_json::json!({"unit": unit, "address": address, "value": value});
    if let Some(h) = hub {
        svc_data["hub"] = serde_json::json!(h);
    }
    match write_type {
        "coil" => {
            if value.is_boolean() {
                // single coil write
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
                // single register write
            } else if let Some(arr) = value.as_array() {
                if arr.is_empty() {
                    return Err("value array must not be empty for holding register writes".into());
                }
                if !arr.iter().all(|v| v.is_number()) {
                    return Err("value array elements must all be numbers for holding register writes".into());
                }
            } else {
                return Err("value must be a number or array of numbers for holding register writes".into());
            }
            call_service(base, "modbus", "write_register", Some(&svc_data))
        }
        _ => Err(format!("write_type must be 'coil' or 'holding', got '{}'", write_type)),
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
    call_service(base, "persistent_notification", "dismiss", Some(&serde_json::json!({"notification_id": notification_id})))
}

pub fn check_config(base: &str, ssh: Option<&SshConfig>) -> Result<String, String> {
    if let Some(out) = shell::try_shell("check_config", ssh, |cfg| shell::ha_cli(cfg, "core check"))? {
        return Ok(out);
    }
    ha_post(base, "/api/config/core/check_config", Some("{}"))
}

pub fn get_error_log(
    base: &str,
    tail_lines: Option<u32>,
    ssh: Option<&SshConfig>,
    log_path: Option<&str>,
) -> Result<String, String> {
    if let Some(n) = tail_lines {
        if n == 0 {
            return Err("tail_lines must be >= 1".into());
        }
    }
    // Prefer shell-backed tail for efficiency and to bypass REST truncation.
    if let Some(out) = shell::try_shell("get_error_log", ssh, |cfg| {
        let path = log_path.unwrap_or(DEFAULT_HA_LOG_PATH);
        let lines = tail_lines.unwrap_or(DEFAULT_SHELL_TAIL_LINES);
        shell::tail_file(cfg, path, lines)
    })? {
        return Ok(out);
    }
    if log_path.is_some() {
        host::log(
            host::LogLevel::Warn,
            "log_path ignored: REST API /api/error_log always returns the default log. \
             Pass ssh to use a custom log_path via shell.",
        );
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

pub fn restart_ha(base: &str, ssh: Option<&SshConfig>) -> Result<String, String> {
    // Destructive action: use strict variant so shell-path errors propagate
    // instead of silently falling back to a REST restart the user didn't ask for.
    if let Some(out) = shell::try_shell_strict("restart_ha", ssh, |cfg| {
        shell::ha_cli(cfg, "core restart")
    })? {
        return Ok(out);
    }
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

/// Convert days-since-Unix-epoch to (year, month, day) in the proleptic
/// Gregorian calendar. Implements the `civil_from_days` algorithm from
/// Howard Hinnant's "chrono-Compatible Low-Level Date Algorithms" (2013):
/// https://howardhinnant.github.io/date_algorithms.html
/// The integer constants below (719468, 146097, 36524, 1460, 153, …) are
/// Hinnant's algorithm constants — do NOT replace them with named
/// constants, they only make sense as a group.
fn days_to_ymd(days_since_epoch: i64) -> (i64, u32, u32) {
    let z = days_since_epoch + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_encode() {
        assert_eq!(url_encode("hello world"), "hello%20world");
        assert_eq!(url_encode("a/b"), "a%2Fb");
        assert_eq!(url_encode("simple"), "simple");
    }

    #[test]
    fn test_validate_entity_id() {
        assert!(validate_entity_id("light.living_room").is_ok());
        assert!(validate_entity_id("sensor.temp-1").is_ok());
        assert!(validate_entity_id("").is_err());
        assert!(validate_entity_id("nodot").is_err());
        assert!(validate_entity_id("bad;id.x").is_err());
    }

    #[test]
    fn test_validate_domain() {
        assert!(validate_domain("light").is_ok());
        assert!(validate_domain("media_player").is_ok());
        assert!(validate_domain("").is_err());
        assert!(validate_domain("bad.domain").is_err());
    }

    #[test]
    fn test_validate_service() {
        assert!(validate_service("turn_on").is_ok());
        assert!(validate_service("turn-on").is_ok());
        assert!(validate_service("").is_err());
        assert!(validate_service("bad service").is_err());
    }

    #[test]
    fn test_validate_iso_prefix() {
        assert!(validate_iso_prefix("2024-01-15T10:30:00Z", "test").is_ok());
        assert!(validate_iso_prefix("not-a-date", "test").is_err());
        assert!(validate_iso_prefix("short", "test").is_err());
    }

    #[test]
    fn test_validate_iso_prefix_rejects_invalid_month_day() {
        assert!(validate_iso_prefix("2024-00-15T10:30:00Z", "test").is_err());
        assert!(validate_iso_prefix("2024-13-15T10:30:00Z", "test").is_err());
        assert!(validate_iso_prefix("2024-01-00T10:30:00Z", "test").is_err());
        assert!(validate_iso_prefix("2024-01-32T10:30:00Z", "test").is_err());
        assert!(validate_iso_prefix("2024-99-99T10:30:00Z", "test").is_err());
        assert!(validate_iso_prefix("2024-12-31T10:30:00Z", "test").is_ok());
    }

    #[test]
    fn test_validate_ha_url() {
        assert!(validate_ha_url("http://192.168.1.100:8123").is_ok());
        assert!(validate_ha_url("http://homeassistant.local:8123").is_ok());
        assert!(validate_ha_url("https://ha.duckdns.org").is_ok());
        assert!(validate_ha_url("http://10.0.0.1:8123").is_ok());
        assert!(validate_ha_url("http://172.16.0.1:8123").is_ok());
        assert!(validate_ha_url("http://localhost:8123").is_ok());
        assert!(validate_ha_url("http://127.0.0.1:8123").is_ok());
        assert!(validate_ha_url("https://my.nabu.casa").is_ok());
        assert!(validate_ha_url("http://myha.internal:8123").is_ok());
        assert!(validate_ha_url("http://attacker.com").is_err());
        assert!(validate_ha_url("http://evil.example.org").is_err());
        assert!(validate_ha_url("ftp://192.168.1.1").is_err());
        assert!(validate_ha_url("not-a-url").is_err());
        assert!(validate_ha_url("http://192.168.1.1.evil.com").is_err());
        assert!(validate_ha_url("http://10.0.0.1.attacker.com").is_err());
        assert!(validate_ha_url("http://172.16.0.1.evil.com").is_err());
        assert!(validate_ha_url("https://https://foo.local").is_err());
    }

    #[test]
    fn test_validate_ha_url_rejects_userinfo_query_fragment() {
        assert!(validate_ha_url("http://attacker.com@192.168.1.1/").is_err());
        assert!(validate_ha_url("http://192.168.1.1@evil.com/").is_err());
        assert!(validate_ha_url("http://192.168.1.1?x=1").is_err());
        assert!(validate_ha_url("http://192.168.1.1#frag").is_err());
        assert!(validate_ha_url("http://user:pass@192.168.1.1/").is_err());
    }

    #[test]
    fn test_normalize_url_lowercases_scheme_and_host() {
        assert_eq!(normalize_url("HTTP://HA:8123"), "http://ha:8123");
        assert_eq!(normalize_url("HTTPS://Ha.Local:8123/"), "https://ha.local:8123");
        // Path case is preserved.
        assert_eq!(normalize_url("http://HA:8123/API/Foo"), "http://ha:8123/API/Foo");
    }

    #[test]
    fn test_is_ip_only() {
        assert!(is_ip_only("192.168.1.1"));
        assert!(is_ip_only("10.0.0.1"));
        assert!(is_ip_only("172.16.0.1"));
        assert!(is_ip_only("255.255.255.255"));
        assert!(!is_ip_only(""));
        assert!(!is_ip_only("."));
        assert!(!is_ip_only("..."));
        assert!(!is_ip_only("192.168."));
        assert!(!is_ip_only("192.168.1"));
        assert!(!is_ip_only("192.168.1.1.1"));
        assert!(!is_ip_only("192.168.1.1.evil.com"));
        assert!(!is_ip_only("abc.def.ghi.jkl"));
        assert!(!is_ip_only("192..168.1"));
    }

    #[test]
    fn test_validate_event_type() {
        assert!(validate_event_type("custom_event").is_ok());
        assert!(validate_event_type("my.event").is_ok());
        assert!(validate_event_type("event123").is_ok());
        assert!(validate_event_type("my-integration-event").is_ok());
        assert!(validate_event_type("").is_err());
        assert!(validate_event_type("bad/event").is_err());
        assert!(validate_event_type("bad event").is_err());
        assert!(validate_event_type(&"x".repeat(256)).is_err());
    }

    #[test]
    fn test_validate_not_empty() {
        assert!(validate_not_empty("value", "field").is_ok());
        assert!(validate_not_empty("", "field").is_err());
    }

    #[test]
    fn test_normalize_url() {
        assert_eq!(normalize_url("http://ha:8123/"), "http://ha:8123");
        assert_eq!(normalize_url("http://ha:8123"), "http://ha:8123");
    }

    #[test]
    fn test_reload_config_entry_validation() {
        // Input validation must fail before any host::http_request call, so
        // these assertions exercise only the local validator paths.
        let base = "http://192.168.1.1:8123";
        assert!(reload_config_entry(base, "").unwrap_err().contains("must not be empty"));
        assert!(reload_config_entry(base, "bad/id").unwrap_err().contains("invalid character"));
        assert!(reload_config_entry(base, "bad id").unwrap_err().contains("invalid character"));
        assert!(reload_config_entry(base, &"a".repeat(MAX_ENTITY_ID_LEN + 1)).unwrap_err().contains("too long"));
    }

    #[test]
    fn test_compact_entity_drops_attributes() {
        let full = serde_json::json!({
            "entity_id": "light.living_room",
            "state": "on",
            "last_changed": "2024-01-15T10:30:00+00:00",
            "last_updated": "2024-01-15T10:30:00+00:00",
            "attributes": {"brightness": 200, "rgb_color": [255, 100, 50]},
            "context": {"id": "abc", "parent_id": null, "user_id": null}
        });
        let c = compact_entity(&full);
        let obj = c.as_object().expect("compact must be object");
        assert_eq!(obj.len(), 3, "compact must keep only entity_id, state, last_changed");
        assert_eq!(obj["entity_id"], "light.living_room");
        assert_eq!(obj["state"], "on");
        assert_eq!(obj["last_changed"], "2024-01-15T10:30:00+00:00");
        assert!(!obj.contains_key("attributes"));
        assert!(!obj.contains_key("context"));
        assert!(!obj.contains_key("last_updated"));
    }

    #[test]
    fn test_compact_entity_handles_missing_fields() {
        let minimal = serde_json::json!({"entity_id": "sensor.x", "state": "42"});
        let c = compact_entity(&minimal);
        let obj = c.as_object().unwrap();
        assert_eq!(obj.len(), 2);
        assert_eq!(obj["entity_id"], "sensor.x");
        assert!(!obj.contains_key("last_changed"));
    }

    #[test]
    fn test_truncate_template_output_under_cap() {
        let s = "small body".to_string();
        assert_eq!(truncate_template_output(s.clone(), 100), s);
    }

    #[test]
    fn test_truncate_template_output_appends_marker() {
        let s = "a".repeat(20);
        let out = truncate_template_output(s, 10);
        assert!(out.starts_with("aaaaaaaaaa"));
        assert!(out.contains("truncated, 10 more bytes"));
        assert!(out.contains("max_chars"));
    }

    #[test]
    fn test_truncate_template_output_respects_utf8_boundaries() {
        // "é" is 2 bytes (0xc3 0xa9). Cap of 1 must back off to 0 to avoid
        // splitting the codepoint.
        let s = "é".to_string();
        let out = truncate_template_output(s, 1);
        // We retained zero bytes of content, then appended the marker.
        assert!(out.starts_with("\n…[truncated, 2 more bytes"));
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

    #[test]
    fn test_days_to_ymd() {
        assert_eq!(days_to_ymd(0), (1970, 1, 1));
        assert_eq!(days_to_ymd(365), (1971, 1, 1));
        assert_eq!(days_to_ymd(19723), (2024, 1, 1));
    }
}

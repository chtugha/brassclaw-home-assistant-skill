pub const MAX_ENTITY_ID_LEN: usize = 255;
pub const MAX_EVENT_TYPE_LEN: usize = 255;
pub const MAX_STATE_LEN: usize = 255;
pub const MAX_TEMPLATE_LEN: usize = 65_536;
pub const MAX_MQTT_TOPIC_LEN: usize = 65_535;

pub fn url_encode(s: &str) -> String {
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

pub fn validate_ha_url(ha_url: &str) -> Result<(), String> {
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
            "ha_url host '{}' is not a recognized Home Assistant address. \
             Accepted: *.nabu.casa, *.duckdns.org (public HTTPS — work through ha-tool REST); \
             localhost, 192.168.*, 10.*, 172.16-31.*, *.local, *.lan, *.home \
             (local — sandbox blocks these; use native `shell` tool with `curl` instead)",
            host_no_port
        ));
    }
    Ok(())
}

fn is_ip_only(s: &str) -> bool {
    if s.is_empty() || s.starts_with('.') || s.ends_with('.') || s.contains("..") {
        return false;
    }
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 4 {
        return false;
    }
    parts
        .iter()
        .all(|p| !p.is_empty() && p.len() <= 3 && p.bytes().all(|b| b.is_ascii_digit()))
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

pub fn normalize_url(ha_url: &str) -> String {
    let trimmed = ha_url.trim_end_matches('/');
    let (scheme_len, scheme) = if trimmed.len() >= 7 && trimmed[..7].eq_ignore_ascii_case("http://")
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

pub fn validate_entity_id(id: &str) -> Result<(), String> {
    if id.is_empty() {
        return Err("entity_id must not be empty".into());
    }
    if !id.contains('.') {
        return Err(format!(
            "entity_id '{}' must contain a dot (e.g. 'light.living_room')",
            id
        ));
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

pub fn validate_domain(d: &str) -> Result<(), String> {
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

pub fn validate_service(s: &str) -> Result<(), String> {
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

pub fn validate_iso_prefix(s: &str, field: &str) -> Result<(), String> {
    let b = s.as_bytes();
    if b.len() < 11
        || !b[0..4].iter().all(|c| c.is_ascii_digit())
        || b[4] != b'-'
        || !b[5..7].iter().all(|c| c.is_ascii_digit())
        || b[7] != b'-'
        || !b[8..10].iter().all(|c| c.is_ascii_digit())
        || b[10] != b'T'
    {
        return Err(format!(
            "{} must be ISO 8601 format (YYYY-MM-DDThh:mm:ss)",
            field
        ));
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

pub fn validate_event_type(s: &str) -> Result<(), String> {
    if s.is_empty() || s.len() > MAX_EVENT_TYPE_LEN {
        return Err(format!(
            "event_type must be 1-{} characters",
            MAX_EVENT_TYPE_LEN
        ));
    }
    for c in s.chars() {
        if !c.is_alphanumeric() && c != '_' && c != '.' && c != '-' {
            return Err(format!("event_type contains invalid character '{}'", c));
        }
    }
    Ok(())
}

pub fn validate_not_empty(value: &str, field: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err(format!("{} must not be empty", field));
    }
    Ok(())
}

pub fn compact_entity(e: &serde_json::Value) -> serde_json::Value {
    let entity_id = e
        .get("entity_id")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
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

pub fn truncate_template_output(raw: String, cap: usize) -> String {
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

pub fn days_to_ymd(days_since_epoch: i64) -> (i64, u32, u32) {
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
        assert!(validate_ha_url("https://abc123def.ui.nabu.casa").is_ok());
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
        assert_eq!(
            normalize_url("HTTPS://Ha.Local:8123/"),
            "https://ha.local:8123"
        );
        assert_eq!(
            normalize_url("http://HA:8123/API/Foo"),
            "http://ha:8123/API/Foo"
        );
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
        assert_eq!(obj.len(), 3);
        assert_eq!(obj["entity_id"], "light.living_room");
        assert_eq!(obj["state"], "on");
        assert_eq!(obj["last_changed"], "2024-01-15T10:30:00+00:00");
    }

    #[test]
    fn test_compact_entity_handles_missing_fields() {
        let minimal = serde_json::json!({"entity_id": "sensor.x", "state": "42"});
        let c = compact_entity(&minimal);
        let obj = c.as_object().unwrap();
        assert_eq!(obj.len(), 2);
        assert_eq!(obj["entity_id"], "sensor.x");
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
        let s = "é".to_string();
        let out = truncate_template_output(s, 1);
        assert!(out.starts_with("\n…[truncated, 2 more bytes"));
    }

    #[test]
    fn test_days_to_ymd() {
        assert_eq!(days_to_ymd(0), (1970, 1, 1));
        assert_eq!(days_to_ymd(365), (1971, 1, 1));
        assert_eq!(days_to_ymd(19723), (2024, 1, 1));
    }
}

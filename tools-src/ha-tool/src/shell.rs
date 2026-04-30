use crate::near::agent::host;
use schemars::JsonSchema;
use serde::Deserialize;

const REMOTE_SHELL_ALIAS: &str = "remote-shell";
const MAX_COMMAND_LEN: usize = 65_536;
const MAX_PATH_LEN: usize = 4096;
const MAX_FILE_WRITE_LEN: usize = 32_768;
const DEFAULT_EXEC_TIMEOUT_SECS: u32 = 60;
const MIN_EXEC_TIMEOUT_SECS: u32 = 1;
const MAX_EXEC_TIMEOUT_SECS: u32 = 3600;
const READ_EXEC_TIMEOUT_SECS: u32 = 30;
const HA_CLI_EXEC_TIMEOUT_SECS: u32 = 300;
const MAX_SSH_HOST_LEN: usize = 253;
const MAX_SSH_USERNAME_LEN: usize = 256;
const MAX_HA_CLI_ARGS_LEN: usize = 2048;
const MAX_TAIL_LINES: u32 = 100_000;

/// SSH connection parameters accepted on every shell-backed action.
///
/// Either reuse an existing `session_id` from a prior `connect` call, or
/// provide full credentials so the ha-tool can open a session on demand.
#[derive(Debug, Deserialize, JsonSchema, Clone)]
pub struct SshConfig {
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub private_key_pem: Option<String>,
    #[serde(default)]
    pub host_key_fingerprint: Option<String>,
    #[serde(default)]
    pub insecure_ignore_host_key: Option<bool>,
    #[serde(default)]
    pub gateway_port: Option<u16>,
}

/// Probe whether the remote-shell extension is installed and reachable.
///
/// Invokes the `health` action against the same gateway port the actual
/// shell command would use. Any Ok response means the sibling tool is
/// usable. `health` is the cheapest probe exposed by the remote-shell
/// extension and has no per-session side effects.
pub fn is_shell_available(gateway_port: Option<u16>) -> bool {
    let mut body = serde_json::json!({"action": "health"});
    if let Some(p) = gateway_port {
        body["gateway_port"] = serde_json::json!(p);
    }
    let p = serde_json::to_string(&body)
        .expect("serializing a static json object is infallible");
    host::tool_invoke(REMOTE_SHELL_ALIAS, &p).is_ok()
}

const SANDBOX_HINT: &str = "The sandbox blocks WASM-to-WASM HTTP calls to local/private \
addresses. For local HA instances, use the native `shell` tool with `curl` instead of ha-tool \
(e.g. shell: curl -s -H 'Authorization: Bearer TOKEN' http://HA:8123/api/states).";

fn is_sandbox_error(err: &str) -> bool {
    let lower = err.to_lowercase();
    lower.contains("http not allowed")
        || lower.contains("insecurescheme")
        || lower.contains("hostnotallowed")
        || lower.contains("private ip")
        || lower.contains("dns rebinding")
}

fn log_shell_fallback(action: &str, reason: &str) {
    host::log(
        host::LogLevel::Warn,
        &format!(
            "shell path unavailable for action '{}': {} — falling back to REST API",
            action, reason
        ),
    );
}

/// Try the shell-backed implementation if `ssh` is provided and remote-shell
/// is installed. On failure or absence, logs a warning and returns Ok(None)
/// so the caller can fall back to REST.
pub fn try_shell<F>(action: &str, ssh: Option<&SshConfig>, f: F) -> Result<Option<String>, String>
where
    F: FnOnce(&SshConfig) -> Result<String, String>,
{
    let Some(cfg) = ssh else { return Ok(None) };
    if !is_shell_available(cfg.gateway_port) {
        log_shell_fallback(action, "remote-shell extension not installed");
        return Ok(None);
    }
    match f(cfg) {
        Ok(s) => Ok(Some(s)),
        Err(e) => {
            if is_sandbox_error(&e) {
                host::log(
                    host::LogLevel::Warn,
                    &format!(
                        "Shell action '{}' blocked by sandbox: {}. {}",
                        action, e, SANDBOX_HINT
                    ),
                );
            } else {
                log_shell_fallback(action, &e);
            }
            Ok(None)
        }
    }
}

/// Strict variant of `try_shell` for destructive actions (e.g. `restart_ha`).
/// Only falls back to REST when remote-shell is NOT installed; propagates any
/// shell execution error instead of silently routing to REST, so users who
/// explicitly supplied SSH credentials don't get an unintended REST restart.
pub fn try_shell_strict<F>(
    action: &str,
    ssh: Option<&SshConfig>,
    f: F,
) -> Result<Option<String>, String>
where
    F: FnOnce(&SshConfig) -> Result<String, String>,
{
    let Some(cfg) = ssh else { return Ok(None) };
    if !is_shell_available(cfg.gateway_port) {
        log_shell_fallback(action, "remote-shell extension not installed");
        return Ok(None);
    }
    f(cfg).map(Some).map_err(|e| {
        if is_sandbox_error(&e) {
            format!(
                "Shell action '{}' blocked by sandbox: {}. {}",
                action, e, SANDBOX_HINT
            )
        } else {
            e
        }
    })
}

fn ensure_session(ssh: &SshConfig) -> Result<String, String> {
    if let Some(sid) = &ssh.session_id {
        if !sid.is_empty() {
            return Ok(sid.clone());
        }
    }
    let host = ssh
        .host
        .as_deref()
        .ok_or("ssh.host required when session_id is not provided")?;
    let username = ssh
        .username
        .as_deref()
        .ok_or("ssh.username required when session_id is not provided")?;
    if host.is_empty() || host.len() > MAX_SSH_HOST_LEN {
        return Err(format!("ssh.host must be 1-{} characters", MAX_SSH_HOST_LEN));
    }
    if username.is_empty() || username.len() > MAX_SSH_USERNAME_LEN {
        return Err(format!("ssh.username must be 1-{} characters", MAX_SSH_USERNAME_LEN));
    }
    let auth = if let Some(pw) = &ssh.password {
        serde_json::json!({"type": "password", "password": pw})
    } else if let Some(key) = &ssh.private_key_pem {
        serde_json::json!({"type": "private_key", "key_pem": key})
    } else {
        return Err("ssh requires password or private_key_pem when opening a new session".into());
    };
    let mut body = serde_json::json!({
        "action": "connect",
        "host": host,
        "username": username,
        "auth": auth,
    });
    if let Some(p) = ssh.port {
        body["port"] = serde_json::json!(p);
    }
    if let Some(fp) = &ssh.host_key_fingerprint {
        body["host_key_fingerprint"] = serde_json::json!(fp);
    }
    if let Some(true) = ssh.insecure_ignore_host_key {
        body["insecure_ignore_host_key"] = serde_json::json!(true);
    }
    if let Some(p) = ssh.gateway_port {
        body["gateway_port"] = serde_json::json!(p);
    }
    let params = serde_json::to_string(&body).map_err(|e| e.to_string())?;
    let resp = host::tool_invoke(REMOTE_SHELL_ALIAS, &params).map_err(|e| {
        if is_sandbox_error(&e) {
            format!("SSH session blocked by sandbox: {}. {}", e, SANDBOX_HINT)
        } else {
            e
        }
    })?;
    parse_connect_response(&resp)
}

/// Extract the session id from the human-formatted `connect` response
/// produced by the remote-shell extension. The expected format is a
/// multi-line string with a `Session ID: <id>` line; any extra lines
/// (greeting, message, hint) are ignored so this stays forward-compatible
/// with cosmetic changes to the response template.
fn parse_connect_response(raw: &str) -> Result<String, String> {
    for line in raw.lines() {
        if let Some(rest) = line.trim_start().strip_prefix("Session ID:") {
            let id = rest.trim();
            if !id.is_empty() {
                return Ok(id.to_string());
            }
        }
    }
    Err(format!(
        "remote-shell connect response missing 'Session ID:' line: {}",
        raw
    ))
}

/// Run a command over SSH. `timeout_secs` is clamped to the gateway's 1..=3600 range.
pub fn shell_exec(ssh: &SshConfig, command: &str, timeout_secs: Option<u32>) -> Result<String, String> {
    if command.is_empty() {
        return Err("command must not be empty".into());
    }
    if command.len() > MAX_COMMAND_LEN {
        return Err(format!("command too long (max {} bytes)", MAX_COMMAND_LEN));
    }
    if command.contains('\0') {
        return Err("command must not contain null bytes".into());
    }
    let session_id = ensure_session(ssh)?;
    let timeout = timeout_secs
        .unwrap_or(DEFAULT_EXEC_TIMEOUT_SECS)
        .clamp(MIN_EXEC_TIMEOUT_SECS, MAX_EXEC_TIMEOUT_SECS);
    let mut body = serde_json::json!({
        "action": "execute",
        "session_id": session_id,
        "command": command,
        "timeout_secs": timeout,
    });
    if let Some(p) = ssh.gateway_port {
        body["gateway_port"] = serde_json::json!(p);
    }
    let params = serde_json::to_string(&body).map_err(|e| e.to_string())?;
    host::tool_invoke(REMOTE_SHELL_ALIAS, &params).map_err(|e| {
        if is_sandbox_error(&e) {
            format!("Shell command blocked by sandbox: {}. {}", e, SANDBOX_HINT)
        } else {
            e
        }
    })
}

/// Parse the human-formatted `execute` response produced by the
/// remote-shell extension into `(exit_code, stdout, stderr)`.
///
/// Expected shapes (see `format_execute_response` in remote-shell):
///
/// ```text
/// Exit code: 0
/// (no output)
/// ```
/// or
/// ```text
/// Exit code: <n>
///
/// --- stdout ---
/// <stdout body>
/// --- stderr ---
/// <stderr body>
/// ```
///
/// Either block may be absent. `Exit code: unknown ...` (timeout) maps
/// to `-1` so call sites observe a non-zero exit.
fn parse_exec_output(raw: &str) -> Result<(i32, String, String), String> {
    const STDOUT_MARKER: &str = "--- stdout ---\n";
    const STDERR_MARKER: &str = "--- stderr ---\n";

    let (header, rest) = raw.split_once('\n').unwrap_or((raw, ""));
    let exit_str = header.strip_prefix("Exit code: ").ok_or_else(|| {
        format!(
            "invalid shell response: expected 'Exit code: ...' header, got {:?}",
            header
        )
    })?;
    let exit_code: i32 = if exit_str.starts_with("unknown") {
        -1
    } else {
        exit_str
            .parse()
            .map_err(|e| format!("invalid exit code '{}': {}", exit_str, e))?
    };

    // Empty body or a body that starts with the "(no output)" sentinel
    // (possibly followed by gateway-side trailer lines) means no streams.
    if rest.is_empty()
        || rest == "(no output)"
        || rest.starts_with("(no output)\n")
    {
        return Ok((exit_code, String::new(), String::new()));
    }

    let stdout_pos = rest.find(STDOUT_MARKER);
    let stderr_pos = rest.find(STDERR_MARKER);
    if stdout_pos.is_none() && stderr_pos.is_none() {
        return Err(format!(
            "invalid shell response: body has neither stdout/stderr markers nor '(no output)': {:?}",
            rest
        ));
    }

    let extract = |start: Option<usize>, marker_len: usize, other: Option<usize>| -> String {
        let Some(s) = start else { return String::new() };
        let content_start = s + marker_len;
        let content_end = match other {
            Some(o) if o > s => o,
            _ => rest.len(),
        };
        let slice = &rest[content_start..content_end];
        match other {
            Some(o) if o > s => slice.strip_suffix('\n').unwrap_or(slice).to_string(),
            _ => slice.to_string(),
        }
    };

    let stdout = extract(stdout_pos, STDOUT_MARKER.len(), stderr_pos);
    let stderr = extract(stderr_pos, STDERR_MARKER.len(), stdout_pos);
    Ok((exit_code, stdout, stderr))
}

fn validate_path(path: &str) -> Result<(), String> {
    if path.is_empty() {
        return Err("path must not be empty".into());
    }
    if path.len() > MAX_PATH_LEN {
        return Err(format!("path too long (max {} bytes)", MAX_PATH_LEN));
    }
    if path.contains('\0') {
        return Err("path must not contain null bytes".into());
    }
    if path.contains('\n') || path.contains('\r') {
        return Err("path must not contain newlines".into());
    }
    if path.contains('\'') {
        return Err("path must not contain single quotes (shell-quoting constraint)".into());
    }
    Ok(())
}

/// Read a file over SSH via `cat`.
pub fn read_file(ssh: &SshConfig, path: &str) -> Result<String, String> {
    validate_path(path)?;
    let raw = shell_exec(ssh, &format!("cat '{}'", path), Some(READ_EXEC_TIMEOUT_SECS))?;
    let (code, stdout, stderr) = parse_exec_output(&raw)?;
    if code != 0 {
        return Err(format!("cat failed (exit {}): {}", code, stderr.trim()));
    }
    Ok(serde_json::json!({"path": path, "content": stdout}).to_string())
}

/// Write a file over SSH atomically: stream via base64 -> tee with sudo fallback off.
pub fn write_file(ssh: &SshConfig, path: &str, content: &str) -> Result<String, String> {
    validate_path(path)?;
    if content.len() > MAX_FILE_WRITE_LEN {
        return Err(format!(
            "content too large ({} bytes; max {} bytes per shell command). \
             Split larger writes into multiple chunks.",
            content.len(),
            MAX_FILE_WRITE_LEN
        ));
    }
    let b64 = b64_encode(content.as_bytes());
    let command = format!("printf %s '{}' | base64 -d > '{}'", b64, path);
    let raw = shell_exec(ssh, &command, Some(DEFAULT_EXEC_TIMEOUT_SECS))?;
    let (code, _stdout, stderr) = parse_exec_output(&raw)?;
    if code != 0 {
        return Err(format!("write failed (exit {}): {}", code, stderr.trim()));
    }
    Ok(serde_json::json!({"path": path, "bytes_written": content.len()}).to_string())
}

/// Tail last N lines of a file.
pub fn tail_file(ssh: &SshConfig, path: &str, lines: u32) -> Result<String, String> {
    validate_path(path)?;
    if lines == 0 || lines > MAX_TAIL_LINES {
        return Err(format!("lines must be between 1 and {}", MAX_TAIL_LINES));
    }
    let raw = shell_exec(ssh, &format!("tail -n {} '{}'", lines, path), Some(READ_EXEC_TIMEOUT_SECS))?;
    let (code, stdout, stderr) = parse_exec_output(&raw)?;
    if code != 0 {
        return Err(format!("tail failed (exit {}): {}", code, stderr.trim()));
    }
    Ok(serde_json::json!({"path": path, "lines": lines, "content": stdout}).to_string())
}

/// Run the Home Assistant `ha` supervisor CLI over SSH.
///
/// Unlike `read_file` / `write_file` / `tail_file`, this returns the raw
/// human-formatted `shell_exec` response (e.g.
/// `"Exit code: 0\n--- stdout ---\n..."`) verbatim so that the agent can
/// surface the full output (including timeout indicators and stderr) to
/// the user. Callers that need structured access to exit code, stdout,
/// and stderr should call `shell_exec` directly and parse the response
/// themselves with the same wire format documented on
/// `parse_exec_output`.
pub fn ha_cli(ssh: &SshConfig, args: &str) -> Result<String, String> {
    if args.is_empty() {
        return Err("args must not be empty (e.g. 'core check', 'core restart', 'core logs')".into());
    }
    if args.len() > MAX_HA_CLI_ARGS_LEN {
        return Err(format!("args too long (max {} bytes)", MAX_HA_CLI_ARGS_LEN));
    }
    // Whitelist: only alphanumerics, space, and a small set of safe punctuation.
    // This prevents quoting/globbing/continuation/injection beyond the `ha` CLI.
    for c in args.chars() {
        let ok = c.is_ascii_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.' | '=' | ':' | ',' | '/');
        if !ok {
            return Err(format!("args contains forbidden character '{}'", c));
        }
    }
    shell_exec(ssh, &format!("ha {}", args), Some(HA_CLI_EXEC_TIMEOUT_SECS))
}

/// Status snapshot: which shell integration is present.
///
/// Pass `gateway_port` (typically from the caller's `SshConfig.gateway_port`)
/// so the probe targets the same gateway that subsequent shell-aware actions
/// will use. Defaults to the standard remote-shell port when `None`.
pub fn shell_status(gateway_port: Option<u16>) -> Result<String, String> {
    let available = is_shell_available(gateway_port);
    Ok(serde_json::json!({
        "remote_shell_available": available,
        "alias": REMOTE_SHELL_ALIAS,
        "note": if available {
            "remote-shell extension is installed; ha-tool shell-backed actions are enabled."
        } else {
            "remote-shell extension not installed. ha-tool falls back to REST-only operation."
        }
    })
    .to_string())
}

/// Tiny base64 encoder (RFC 4648 standard alphabet, no line breaks).
fn b64_encode(input: &[u8]) -> String {
    const ALPHA: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((input.len() + 2) / 3 * 4);
    let mut i = 0;
    while i + 3 <= input.len() {
        let n = ((input[i] as u32) << 16) | ((input[i + 1] as u32) << 8) | (input[i + 2] as u32);
        out.push(ALPHA[((n >> 18) & 0x3f) as usize] as char);
        out.push(ALPHA[((n >> 12) & 0x3f) as usize] as char);
        out.push(ALPHA[((n >> 6) & 0x3f) as usize] as char);
        out.push(ALPHA[(n & 0x3f) as usize] as char);
        i += 3;
    }
    let rem = input.len() - i;
    if rem == 1 {
        let n = (input[i] as u32) << 16;
        out.push(ALPHA[((n >> 18) & 0x3f) as usize] as char);
        out.push(ALPHA[((n >> 12) & 0x3f) as usize] as char);
        out.push('=');
        out.push('=');
    } else if rem == 2 {
        let n = ((input[i] as u32) << 16) | ((input[i + 1] as u32) << 8);
        out.push(ALPHA[((n >> 18) & 0x3f) as usize] as char);
        out.push(ALPHA[((n >> 12) & 0x3f) as usize] as char);
        out.push(ALPHA[((n >> 6) & 0x3f) as usize] as char);
        out.push('=');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_b64_encode() {
        assert_eq!(b64_encode(b""), "");
        assert_eq!(b64_encode(b"f"), "Zg==");
        assert_eq!(b64_encode(b"fo"), "Zm8=");
        assert_eq!(b64_encode(b"foo"), "Zm9v");
        assert_eq!(b64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(b64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(b64_encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn test_validate_path() {
        assert!(validate_path("/etc/hosts").is_ok());
        assert!(validate_path("/config/configuration.yaml").is_ok());
        assert!(validate_path("").is_err());
        assert!(validate_path("bad\npath").is_err());
        assert!(validate_path("bad'path").is_err());
        assert!(validate_path("bad\0path").is_err());
    }

    #[test]
    fn test_parse_connect_response_happy_path() {
        let raw = "Connected successfully.\n\
                   Session ID: abc-123-def\n\
                   Connected to host.example:22 as alice.\n\n\
                   Use this session_id for 'execute' and 'disconnect' calls.";
        assert_eq!(parse_connect_response(raw).unwrap(), "abc-123-def");
    }

    #[test]
    fn test_parse_connect_response_missing_line() {
        let raw = "Connected successfully.\nGreetings.";
        let err = parse_connect_response(raw).unwrap_err();
        assert!(err.contains("Session ID"));
    }

    #[test]
    fn test_parse_connect_response_empty_id() {
        let raw = "Connected successfully.\nSession ID: \nfoo";
        assert!(parse_connect_response(raw).is_err());
    }

    #[test]
    fn test_parse_exec_output_no_output() {
        let raw = "Exit code: 0\n(no output)";
        let (code, out, err) = parse_exec_output(raw).unwrap();
        assert_eq!(code, 0);
        assert_eq!(out, "");
        assert_eq!(err, "");
    }

    #[test]
    fn test_parse_exec_output_stdout_only() {
        let raw = "Exit code: 0\n\n--- stdout ---\nhello world\n";
        let (code, out, err) = parse_exec_output(raw).unwrap();
        assert_eq!(code, 0);
        assert_eq!(out, "hello world\n");
        assert_eq!(err, "");
    }

    #[test]
    fn test_parse_exec_output_stderr_only() {
        let raw = "Exit code: 1\n\n--- stderr ---\nboom";
        let (code, out, err) = parse_exec_output(raw).unwrap();
        assert_eq!(code, 1);
        assert_eq!(out, "");
        assert_eq!(err, "boom");
    }

    #[test]
    fn test_parse_exec_output_both_streams() {
        // Canonical wire format from `format_execute_response`: stdout content
        // followed by "\n--- stderr ---\n". When stdout did not already end
        // with '\n', exactly one '\n' sits between the content and the next
        // marker (it's the separator '\n' from the marker template).
        let raw = "Exit code: 2\n\n--- stdout ---\nout-line\n--- stderr ---\nerr-line";
        let (code, out, err) = parse_exec_output(raw).unwrap();
        assert_eq!(code, 2);
        assert_eq!(out, "out-line");
        assert_eq!(err, "err-line");
    }

    #[test]
    fn test_parse_exec_output_both_streams_trailing_newline() {
        // Variant where stdout content already ends with '\n' (e.g. `cat file`
        // output): formatter appends "\n--- stderr ---\n" giving two '\n'
        // chars between content and the marker. The parser strips exactly
        // the separator '\n', preserving the content's own trailing newline.
        let raw = "Exit code: 0\n\n--- stdout ---\nout\n\n--- stderr ---\nerr\n";
        let (code, out, err) = parse_exec_output(raw).unwrap();
        assert_eq!(code, 0);
        assert_eq!(out, "out\n");
        assert_eq!(err, "err\n");
    }

    #[test]
    fn test_parse_exec_output_tolerates_trailing_lines_after_no_output() {
        // F19: a future gateway footer line after `(no output)` should not
        // break the parser.
        let raw = "Exit code: 0\n(no output)\nWarning: command exceeded soft limit";
        let (code, out, err) = parse_exec_output(raw).unwrap();
        assert_eq!(code, 0);
        assert_eq!(out, "");
        assert_eq!(err, "");
    }

    #[test]
    fn test_shell_exec_rejects_null_bytes() {
        let ssh = SshConfig {
            session_id: Some("x".into()),
            host: None,
            port: None,
            username: None,
            password: None,
            private_key_pem: None,
            host_key_fingerprint: None,
            insecure_ignore_host_key: None,
            gateway_port: None,
        };
        assert!(shell_exec(&ssh, "echo bad\0byte", None)
            .unwrap_err()
            .contains("null bytes"));
    }

    #[test]
    fn test_parse_exec_output_unknown_body_is_err() {
        // Defensive: if the body is non-empty, isn't `(no output)`, and
        // contains no recognisable marker, surface it as an error rather
        // than silently returning empty stdout/stderr.
        let raw = "Exit code: 0\nunexpected footer line";
        let err = parse_exec_output(raw).unwrap_err();
        assert!(err.contains("invalid shell response"));
    }

    #[test]
    fn test_parse_exec_output_unknown_exit_code() {
        let raw = "Exit code: unknown (command may have timed out)\n(no output)";
        let (code, out, err) = parse_exec_output(raw).unwrap();
        assert_eq!(code, -1);
        assert_eq!(out, "");
        assert_eq!(err, "");
    }

    #[test]
    fn test_parse_exec_output_invalid_header() {
        let raw = "{\"exit_code\": 0, \"stdout\": \"\", \"stderr\": \"\"}";
        assert!(parse_exec_output(raw).is_err());
    }

    #[test]
    fn test_parse_exec_output_roundtrip_format() {
        // Mirrors `format_execute_response` in the remote-shell extension to
        // catch wire-format drift between the two repos.
        let format = |exit: Option<i32>, stdout: &str, stderr: &str| -> String {
            let exit_str = exit
                .map(|c| c.to_string())
                .unwrap_or_else(|| "unknown (command may have timed out)".into());
            if stdout.is_empty() && stderr.is_empty() {
                return format!("Exit code: {exit_str}\n(no output)");
            }
            let mut s = format!("Exit code: {exit_str}\n");
            if !stdout.is_empty() {
                s.push_str("\n--- stdout ---\n");
                s.push_str(stdout);
            }
            if !stderr.is_empty() {
                s.push_str("\n--- stderr ---\n");
                s.push_str(stderr);
            }
            s
        };

        let cases: &[(Option<i32>, &str, &str)] = &[
            (Some(0), "", ""),
            (Some(0), "ok\n", ""),
            (Some(1), "", "fail"),
            (Some(2), "out", "err"),
            (Some(2), "out\n", "err\n"),
            (None, "", ""),
        ];
        for (exit, stdout, stderr) in cases {
            let raw = format(*exit, stdout, stderr);
            let (code, out, err) = parse_exec_output(&raw).unwrap();
            let expected_code = exit.unwrap_or(-1);
            assert_eq!(code, expected_code, "exit mismatch for {:?}", raw);
            assert_eq!(out, *stdout, "stdout mismatch for {:?}", raw);
            assert_eq!(err, *stderr, "stderr mismatch for {:?}", raw);
        }
    }

    #[test]
    fn test_max_file_write_len_fits_in_command_budget() {
        // The base64-encoded payload + skeleton + worst-case path must fit
        // inside MAX_COMMAND_LEN, otherwise write_file's cap is unreachable.
        let b64_len = (MAX_FILE_WRITE_LEN + 2) / 3 * 4;
        let skeleton = "printf %s '' | base64 -d > ''".len();
        let worst_case = b64_len + skeleton + MAX_PATH_LEN;
        assert!(
            worst_case <= MAX_COMMAND_LEN,
            "MAX_FILE_WRITE_LEN cap unreachable: worst-case command {} > {}",
            worst_case,
            MAX_COMMAND_LEN
        );
    }

    #[test]
    fn test_is_sandbox_error_detection() {
        assert!(is_sandbox_error("HTTP not allowed: HTTP request not allowed: Denied(InsecureScheme(\"http\"))"));
        assert!(is_sandbox_error("Tool error: HostNotAllowed: host 192.168.1.100 not in allowlist"));
        assert!(is_sandbox_error("DNS rebinding detected: homeassistant.local resolved to private IP 192.168.1.100"));
        assert!(is_sandbox_error("Blocked: private IP 10.0.0.1 not allowed"));
        assert!(!is_sandbox_error("Connection refused"));
        assert!(!is_sandbox_error("Timeout after 30s"));
        assert!(!is_sandbox_error("remote-shell extension not installed"));
    }

    #[test]
    fn test_ha_cli_rejects_metacharacters() {
        let ssh = SshConfig {
            session_id: Some("x".into()),
            host: None,
            port: None,
            username: None,
            password: None,
            private_key_pem: None,
            host_key_fingerprint: None,
            insecure_ignore_host_key: None,
            gateway_port: None,
        };
        assert!(ha_cli(&ssh, "core check; rm -rf /").is_err());
        assert!(ha_cli(&ssh, "core check && whoami").is_err());
        assert!(ha_cli(&ssh, "core check | grep x").is_err());
        assert!(ha_cli(&ssh, "").is_err());
        // Whitelist rejects quoting and globbing too.
        assert!(ha_cli(&ssh, "core 'check'").is_err());
        assert!(ha_cli(&ssh, "core check\\").is_err());
        assert!(ha_cli(&ssh, "core *").is_err());
        assert!(ha_cli(&ssh, "core ?").is_err());
        assert!(ha_cli(&ssh, "core (check)").is_err());
        assert!(ha_cli(&ssh, "core {check}").is_err());
    }
}

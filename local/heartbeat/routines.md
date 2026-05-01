<!-- INSTALL_PREAMBLE: This file is a template. {{HA_URL}} and {{HA_TOKEN}}
     placeholders are replaced automatically by local/scripts/install.sh
     during installation. If you are editing this file manually, replace
     every {{HA_URL}} with your HA base URL and {{HA_TOKEN}} with your
     long-lived access token. -->

Ready-to-use prompts for creating scheduled monitoring jobs. Copy any
prompt below, paste it into `ironclaw chat`, and the agent will set up
the cron routine for you.

**These routines support two tool modes:**
- **`http` tool (preferred)** — always available in every context. Requires
  `HTTP_ALLOW_LOCALHOST=true` in the IronClaw environment.
- **`shell` tool (fallback)** — requires `allow_local_tools = true` at
  IronClaw startup.

Each routine below tries the `http` tool first. If it fails with an
SSRF/HTTPS error, it falls back to `shell+curl`. If neither works, the
routine sends a notification and aborts.

## 1. Hourly health check

```
Create a cron routine named "ha-hourly-health" that runs at minute 5 of every
hour. The job should:
0. Tool check: try calling the http tool with
   {"method":"GET","url":"{{HA_URL}}/api/","headers":[{"name":"Authorization","value":"Bearer {{HA_TOKEN}}"}]}
   If http returns an SSRF or HTTPS error, try shell: curl -s -H "Authorization: Bearer {{HA_TOKEN}}" {{HA_URL}}/api/
   If neither tool works, send a notification: "ha-hourly-health: no usable
   tool. Set HTTP_ALLOW_LOCALHOST=true and restart IronClaw." Then abort.
1. GET {{HA_URL}}/api/ — confirm HA is reachable
2. POST {{HA_URL}}/api/config/core/check_config with body {}
3. GET {{HA_URL}}/api/states — filter for persistent_notification.* entities
If any call fails or check_config is not valid or persistent notifications
exist, send a notification with the findings. Otherwise, stay silent.
Never call any write endpoints without explicit user confirmation.
```

## 2. Daily error-log digest

```
Create a cron routine named "ha-daily-errors" that runs every day at 08:00.
The job should:
0. Tool check: try the http tool with a GET to {{HA_URL}}/api/ (with
   Authorization: Bearer {{HA_TOKEN}} header). If http fails with SSRF/HTTPS
   error, fall back to shell+curl. If neither works, notify and abort.
1. GET {{HA_URL}}/api/error_log
2. Extract only ERROR and WARNING lines from the last 24 hours
3. Write the digest to memory at ha/daily-errors/<date>.md
4. Notify only if there are more than 10 errors or any CRITICAL lines
```

## 3. Weekly integration-update scan

```
Create a cron routine named "ha-weekly-updates" that runs every Monday at 09:00.
The job should:
0. Tool check: try the http tool with a GET to {{HA_URL}}/api/ (with
   Authorization: Bearer {{HA_TOKEN}} header). If http fails with SSRF/HTTPS
   error, fall back to shell+curl. If neither works, notify and abort.
1. GET {{HA_URL}}/api/states — filter for update.* entities where state == "on"
2. For each available update, include attributes.title and
   attributes.latest_version in the report
3. Notify the user with the list and ask whether to proceed (updates must be
   triggered by the user — never auto-apply).
```

## 4. Stuck-automation scan

```
Create a cron routine named "ha-automation-health" that runs every 6 hours.
The job should:
0. Tool check: try the http tool with a GET to {{HA_URL}}/api/ (with
   Authorization: Bearer {{HA_TOKEN}} header). If http fails with SSRF/HTTPS
   error, fall back to shell+curl. If neither works, notify and abort.
1. GET {{HA_URL}}/api/states — filter for automation.* entities
2. Flag automations with state="unavailable" or last_triggered older than 30 days
3. Write findings to memory at ha/automation-health/<date>.md
4. Notify only if at least one automation is unavailable
```

## 5. Battery-low sweep

```
Create a cron routine named "ha-battery-check" that runs every day at 18:00.
The job should:
0. Tool check: try the http tool with a GET to {{HA_URL}}/api/ (with
   Authorization: Bearer {{HA_TOKEN}} header). If http fails with SSRF/HTTPS
   error, fall back to shell+curl. If neither works, notify and abort.
1. GET {{HA_URL}}/api/states — filter for entities with device_class "battery" and state < 20
2. Notify the user with a list of devices needing batteries
```

## Remediation Discipline

All routines above are **read-only by design**. Proposed fixes are delivered
as chat notifications. The user confirms in chat, and the agent then calls
the appropriate write endpoint in the normal conversation — not from inside
the routine itself.

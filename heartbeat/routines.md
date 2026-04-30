# Home Assistant Cron Routines

<!-- INSTALL_PREAMBLE: This file is a template. {{HA_URL}} placeholders are
     replaced automatically by scripts/install.sh during installation.
     If you are editing this file manually, replace every {{HA_URL}} with
     your Home Assistant base URL (e.g. http://192.168.1.100:8123). -->

Ready-to-use prompts for creating scheduled monitoring jobs. Copy any
prompt below, paste it into `ironclaw chat`, and the agent will set up
the cron routine for you.

**Note:** All routines below use `ha-tool` syntax. If your HA URL is
local/HTTP (not public HTTPS), the agent must substitute each `ha-tool`
call with the equivalent `curl` command via the native `shell` tool
(see SKILL.md "Local HA via shell+curl" for the mapping).

## 1. Hourly health check

```
Create a cron routine named "ha-hourly-health" that runs at minute 5 of every
hour. The job should:
1. Call ha-tool get_status ha_url={{HA_URL}}
2. Call ha-tool check_config ha_url={{HA_URL}}
3. Call ha-tool get_notifications ha_url={{HA_URL}}
If any call fails or check_config is not valid or persistent notifications
exist, send a notification with the findings. Otherwise, stay silent.
Never call any write actions without explicit user confirmation.
```

## 2. Daily error-log digest

```
Create a cron routine named "ha-daily-errors" that runs every day at 08:00.
The job should:
1. Call ha-tool get_error_log ha_url={{HA_URL}}
2. Extract only ERROR and WARNING lines from the last 24 hours
3. Write the digest to memory at ha/daily-errors/<date>.md
4. Notify only if there are more than 10 errors or any CRITICAL lines
```

## 3. Weekly integration-update scan

```
Create a cron routine named "ha-weekly-updates" that runs every Monday at 09:00.
The job should:
1. Call ha-tool get_states ha_url={{HA_URL}} domain_filter=update
2. List any update.* entity whose state is "on"
3. For each available update, include attributes.title and
   attributes.latest_version in the report
4. Notify the user with the list and ask whether to proceed (updates must be
   triggered by the user — never auto-apply).
```

## 4. Stuck-automation scan

```
Create a cron routine named "ha-automation-health" that runs every 6 hours.
The job should:
1. Call ha-tool get_states ha_url={{HA_URL}} domain_filter=automation
2. Flag automations with state="unavailable" or last_triggered older than 30 days
3. Write findings to memory at ha/automation-health/<date>.md
4. Notify only if at least one automation is unavailable
```

## 5. Battery-low sweep

```
Create a cron routine named "ha-battery-check" that runs every day at 18:00.
The job should:
1. Call ha-tool get_states ha_url={{HA_URL}} domain_filter=sensor
2. Filter for entities where attributes.device_class=="battery" and
   the numeric state is below 20
3. Notify the user with a list of devices needing batteries
```

## Remediation Discipline

All routines above are **read-only by design**. Proposed fixes are delivered
as chat notifications. The user confirms in chat, and the agent then calls
the appropriate ha-tool write action (`reload_*`, `toggle_automation`,
`call_service`, etc.) in the normal conversation — not from inside the
routine itself.

<!-- INSTALL_PREAMBLE: This file is a template. {{HA_URL}} placeholders are
     replaced automatically by local/scripts/install.sh during installation.
     If you are editing this file manually, replace every {{HA_URL}} with
     your Home Assistant base URL (e.g. http://192.168.1.100:8123). -->

Ready-to-use prompts for creating scheduled monitoring jobs. Copy any
prompt below, paste it into `ironclaw chat`, and the agent will set up
the cron routine for you.

All routines use the native `shell` tool with `curl` to call the HA REST API.

## 1. Hourly health check

```
Create a cron routine named "ha-hourly-health" that runs at minute 5 of every
hour. The job should:
1. Run: curl -s -H "Authorization: Bearer $TOKEN" {{HA_URL}}/api/
2. Run: curl -s -X POST -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" -d '{}' {{HA_URL}}/api/config/core/check_config
3. Run: curl -s -H "Authorization: Bearer $TOKEN" {{HA_URL}}/api/states | jq '[.[] | select(.entity_id | startswith("persistent_notification."))]'
If any call fails or check_config is not valid or persistent notifications
exist, send a notification with the findings. Otherwise, stay silent.
Never call any write endpoints without explicit user confirmation.
```

## 2. Daily error-log digest

```
Create a cron routine named "ha-daily-errors" that runs every day at 08:00.
The job should:
1. Run: curl -s -H "Authorization: Bearer $TOKEN" {{HA_URL}}/api/error_log
2. Extract only ERROR and WARNING lines from the last 24 hours
3. Write the digest to memory at ha/daily-errors/<date>.md
4. Notify only if there are more than 10 errors or any CRITICAL lines
```

## 3. Weekly integration-update scan

```
Create a cron routine named "ha-weekly-updates" that runs every Monday at 09:00.
The job should:
1. Run: curl -s -H "Authorization: Bearer $TOKEN" {{HA_URL}}/api/states | jq '[.[] | select(.entity_id | startswith("update.")) | select(.state == "on")]'
2. For each available update, include attributes.title and
   attributes.latest_version in the report
3. Notify the user with the list and ask whether to proceed (updates must be
   triggered by the user — never auto-apply).
```

## 4. Stuck-automation scan

```
Create a cron routine named "ha-automation-health" that runs every 6 hours.
The job should:
1. Run: curl -s -H "Authorization: Bearer $TOKEN" {{HA_URL}}/api/states | jq '[.[] | select(.entity_id | startswith("automation."))]'
2. Flag automations with state="unavailable" or last_triggered older than 30 days
3. Write findings to memory at ha/automation-health/<date>.md
4. Notify only if at least one automation is unavailable
```

## 5. Battery-low sweep

```
Create a cron routine named "ha-battery-check" that runs every day at 18:00.
The job should:
1. Run: curl -s -H "Authorization: Bearer $TOKEN" {{HA_URL}}/api/states | jq '[.[] | select(.attributes.device_class == "battery") | select((.state | tonumber) < 20)]'
2. Notify the user with a list of devices needing batteries
```

## Remediation Discipline

All routines above are **read-only by design**. Proposed fixes are delivered
as chat notifications. The user confirms in chat, and the agent then calls
the appropriate write endpoint in the normal conversation — not from inside
the routine itself.

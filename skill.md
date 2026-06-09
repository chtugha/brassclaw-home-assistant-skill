---
name: home-assistant
version: "2.0.0"
description: Control Home Assistant via HTTP API
activation:
  keywords: ["home assistant", "light", "switch", "thermostat"]
  patterns: ["(?i)(turn|set|check).*(light|switch|temperature)"]
  tags: ["home-automation"]
  max_context_tokens: 3000
credentials:
  - name: ha_url
    provider: homeassistant
    location: {type: header, name: X-HA-URL}
    hosts: ["*"]
    setup_instructions: "Enter your Home Assistant URL"
  - name: ha_token
    provider: homeassistant
    location: {type: bearer}
    hosts: ["*"]
    setup_instructions: "Create long-lived access token"
---
# Home Assistant Skill
Control Home Assistant via HTTP API. Credentials auto-injected.
Use `http` tool with `{ha_url}/api/states` and `/api/services/{domain}/{service}`.

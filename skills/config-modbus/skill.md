---
name: home-assistant-config-modbus
version: "2.0.0"
description: Edit config and Modbus
activation:
  keywords: ["config", "modbus"]
  tags: ["home-automation"]
  max_context_tokens: 3000
credentials:
  - name: ha_url
    provider: homeassistant
    location: {type: header, name: X-HA-URL}
    hosts: ["*"]
  - name: ha_token
    provider: homeassistant
    location: {type: bearer}
    hosts: ["*"]
---
# Config & Modbus
Edit config and probe Modbus.

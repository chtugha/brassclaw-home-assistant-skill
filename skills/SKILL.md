---
name: home-assistant
version: 0.5.2
description: Control Home Assistant via MCP
activation:
  keywords:
    - home assistant
    - homeassistant
    - light
    - switch
    - thermostat
    - climate
    - temperature
  patterns:
    - "turn (on|off|toggle).*(light|switch|fan|plug|outlet)"
  tags:
    - home-automation
  max_context_tokens: 2000
---

# Home Assistant MCP

Use these tools to manage Home Assistant:
1. **`homeassistant_ha_search_entities(query, domain=None)`**: Search for entities, status, or updates.
2. **`homeassistant_ha_control(entity_id, action, value=None)`**: Control devices (turn_on/off, toggle, set_value).
3. **`homeassistant_ha_get_diagnostics()`**: Check health, logs, or updates.

**Instructions for Common Requests**:
- **Updates / System Health**: Call `homeassistant_ha_get_diagnostics()`. Also search for `update` entities with `homeassistant_ha_search_entities(query="update")`.
- **Status of Devices**: Search for the device name/type first to inspect its current state.
- **Control Devices**: Search to find the exact `entity_id` first, then call control action with that ID.

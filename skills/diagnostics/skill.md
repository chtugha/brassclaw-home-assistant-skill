---
name: home-assistant-diagnostics
version: "2.0.0"
description: Check system health
activation:
  keywords: ["diagnostics"]
  tags: ["home-automation"]
  max_context_tokens: 2500
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
# Diagnostics
Check system health.

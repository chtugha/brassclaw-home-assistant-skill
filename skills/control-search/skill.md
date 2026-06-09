---
name: home-assistant-control-search
version: "2.0.0"
description: Search and control devices
activation:
  keywords: ["home assistant", "light"]
  tags: ["home-automation"]
  max_context_tokens: 1500
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
# Control & Search
Search and control devices.

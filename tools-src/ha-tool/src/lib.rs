mod api;
mod shell;
mod types;

use types::HaAction;

wit_bindgen::generate!({
    world: "sandboxed-tool",
    path: "../../wit/tool.wit",
});

struct HaTool;

impl exports::near::agent::tool::Guest for HaTool {
    fn execute(req: exports::near::agent::tool::Request) -> exports::near::agent::tool::Response {
        match execute_inner(&req.params) {
            Ok(result) => exports::near::agent::tool::Response {
                output: Some(result),
                error: None,
            },
            Err(e) => exports::near::agent::tool::Response {
                output: None,
                error: Some(e),
            },
        }
    }

    fn schema() -> String {
        let schema = schemars::schema_for!(types::HaAction);
        serde_json::to_string(&schema).expect("schema serialization is infallible")
    }

    fn description() -> String {
        "Home Assistant integration tool. Control lights, switches, climate, media players, \
         automations, scripts, scenes, MQTT, Modbus, templates, history, logbook, calendar, \
         notifications, reloads (core_config, automations, scripts, scenes, themes, \
         config_entry), and system management (check_config, error_log, restart) via the HA \
         REST API. Requires ha_token (configure with `ironclaw tool auth ha-tool`). \
         Every call requires ha_url parameter with the HA instance base URL."
            .to_string()
    }
}

fn execute_inner(params: &str) -> Result<String, String> {
    if !crate::near::agent::host::secret_exists("ha_token") {
        return Err(
            "Home Assistant token not configured. Run: ironclaw tool auth ha-tool".to_string(),
        );
    }

    let action: HaAction =
        serde_json::from_str(params).map_err(|e| format!("Invalid parameters: {}", e))?;

    crate::near::agent::host::log(
        crate::near::agent::host::LogLevel::Info,
        &format!("Executing HA action: {:?}", action),
    );

    match action {
        HaAction::GetStatus { ha_url } => api::get_status(&ha_url),
        HaAction::GetStates { ha_url, domain_filter, max_items } => {
            api::get_states(&ha_url, domain_filter.as_deref(), max_items)
        }
        HaAction::GetState { ha_url, entity_id } => api::get_state(&ha_url, &entity_id),
        HaAction::SetState { ha_url, entity_id, state, attributes } => {
            api::set_state(&ha_url, &entity_id, &state, attributes.as_ref())
        }
        HaAction::CallService { ha_url, domain, service, data } => {
            api::call_service(&ha_url, &domain, &service, data.as_ref())
        }
        HaAction::GetServices { ha_url } => api::get_services(&ha_url),
        HaAction::FireEvent { ha_url, event_type, event_data } => {
            api::fire_event(&ha_url, &event_type, event_data.as_ref())
        }
        HaAction::RenderTemplate { ha_url, template } => {
            api::render_template(&ha_url, &template)
        }
        HaAction::GetHistory { ha_url, entity_id, hours_back, start_time } => {
            api::get_history(&ha_url, &entity_id, hours_back, start_time.as_deref())
        }
        HaAction::GetLogbook { ha_url, entity_id, hours_back } => {
            api::get_logbook(&ha_url, entity_id.as_deref(), hours_back)
        }
        HaAction::GetCalendarEvents { ha_url, entity_id, start, end } => {
            api::get_calendar_events(&ha_url, &entity_id, &start, &end)
        }
        HaAction::ListAutomations { ha_url } => {
            api::get_states(&ha_url, Some("automation"), None)
        }
        HaAction::ToggleAutomation { ha_url, entity_id, enabled } => {
            api::toggle_automation(&ha_url, &entity_id, enabled)
        }
        HaAction::TriggerAutomation { ha_url, entity_id } => {
            api::trigger_automation(&ha_url, &entity_id)
        }
        HaAction::ListScripts { ha_url } => api::get_states(&ha_url, Some("script"), None),
        HaAction::RunScript { ha_url, entity_id, variables } => {
            api::run_script(&ha_url, &entity_id, variables.as_ref())
        }
        HaAction::ListScenes { ha_url } => api::get_states(&ha_url, Some("scene"), None),
        HaAction::ActivateScene { ha_url, entity_id } => {
            api::activate_scene(&ha_url, &entity_id)
        }
        HaAction::MqttPublish { ha_url, topic, payload, qos, retain } => {
            api::mqtt_publish(&ha_url, &topic, &payload, qos, retain)
        }
        HaAction::ModbusWrite { ha_url, hub, unit, address, value, write_type } => {
            api::modbus_write(&ha_url, hub.as_deref(), unit, address, &value, &write_type)
        }
        HaAction::GetConfig { ha_url } => api::get_config(&ha_url),
        HaAction::GetNotifications { ha_url } => api::get_notifications(&ha_url),
        HaAction::DismissNotification { ha_url, notification_id } => {
            api::dismiss_notification(&ha_url, &notification_id)
        }
        HaAction::CheckConfig { ha_url, ssh } => api::check_config(&ha_url, ssh.as_ref()),
        HaAction::GetErrorLog { ha_url, tail_lines, ssh, log_path } => {
            api::get_error_log(&ha_url, tail_lines, ssh.as_ref(), log_path.as_deref())
        }
        HaAction::RestartHa { ha_url, ssh } => api::restart_ha(&ha_url, ssh.as_ref()),
        HaAction::ShellStatus { gateway_port } => shell::shell_status(gateway_port),
        HaAction::ShellExec { ssh, command, timeout_secs } => {
            shell::shell_exec(&ssh, &command, timeout_secs)
        }
        HaAction::ShellReadFile { ssh, path } => shell::read_file(&ssh, &path),
        HaAction::ShellWriteFile { ssh, path, content } => {
            shell::write_file(&ssh, &path, &content)
        }
        HaAction::ShellTailFile { ssh, path, lines } => shell::tail_file(&ssh, &path, lines),
        HaAction::HaCli { ssh, args } => shell::ha_cli(&ssh, &args),
        HaAction::ReloadCoreConfig { ha_url } => api::reload_core_config(&ha_url),
        HaAction::ReloadAutomations { ha_url } => api::reload_automations(&ha_url),
        HaAction::ReloadScripts { ha_url } => api::reload_scripts(&ha_url),
        HaAction::ReloadScenes { ha_url } => api::reload_scenes(&ha_url),
        HaAction::ReloadThemes { ha_url } => api::reload_themes(&ha_url),
        HaAction::ReloadConfigEntry { ha_url, entry_id } => {
            api::reload_config_entry(&ha_url, &entry_id)
        }
    }
}

export!(HaTool);

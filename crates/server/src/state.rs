use axum::extract::FromRef;

use magpie_telemetry::LogControl;

#[derive(Clone, FromRef)]
pub(crate) struct AppState {
    pub log_control: LogControl,
}

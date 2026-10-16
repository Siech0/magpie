use axum::extract::FromRef;

use magpie_db::Db;
use magpie_telemetry::LogControl;

#[derive(Clone, FromRef)]
pub(crate) struct AppState {
    pub db: Db,
    pub log_control: LogControl,
}

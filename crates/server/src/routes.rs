use axum::Router;

use crate::state::AppState;

/// The complete HTTP surface of the server
pub(crate) fn app(state: AppState) -> Router {
    let routes = Router::new().merge(magpie_http::ops::routes());

    magpie_http::middleware::global(routes).with_state(state)
}

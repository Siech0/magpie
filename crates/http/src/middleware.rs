//! Middleware that applies to every request, outmost first.
use axum::{Router, extract::Request};
use tower::ServiceBuilder;
use tower_http::{
    LatencyUnit,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, RequestId, SetRequestIdLayer},
    trace::{DefaultOnResponse, TraceLayer},
};
use tracing::{Level, Span};

/// Middleware common to all routers
pub fn global<State>(router: Router<State>) -> Router<State>
where
    State: Clone + Send + Sync + 'static,
{
    router.layer(
        ServiceBuilder::new()
            .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
            .layer(PropagateRequestIdLayer::x_request_id())
            .layer(
                TraceLayer::new_for_http()
                    .make_span_with(request_span)
                    .on_response(
                        DefaultOnResponse::new()
                            .level(Level::INFO)
                            .latency_unit(LatencyUnit::Micros),
                    ),
            ),
    )
}

fn request_span(request: &Request) -> Span {
    let request_id = request
        .extensions()
        .get::<RequestId>()
        .and_then(|id| id.header_value().to_str().ok())
        .unwrap_or_default();

    // Only the path is recorded: query strings can carry credentials, such as presigned URLs.
    tracing::info_span!(
        "request",
        http.request.method = %request.method(),
        url.path = request.uri().path(),
        request_id,
        user.id = tracing::field::Empty,
    )
}

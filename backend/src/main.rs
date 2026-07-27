use axum::http::header::{AUTHORIZATION, CONTENT_TYPE};
use axum::http::Method;
use axum::{routing::get, Router};
use me_doc_backend::{auth, collab, comments, config, db, email::EmailClient, export, health, pages, sharing, storage, users, versions, workspaces, AppState};
use redis::Client as RedisClient;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_cookies::CookieManagerLayer;
use tower_governor::{governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor, GovernorLayer};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let config = config::Config::from_env()?;

    let db = db::create_pool(&config.database_url).await?;
    sqlx::migrate!("./migrations").run(&db).await?;

    let redis = RedisClient::open(config.redis_url.clone())?;
    let email = EmailClient::new(&config.smtp_host, config.smtp_port, &config.smtp_from)?;
    let s3 = storage::build_client(&config);
    let s3_presign = storage::build_presign_client(&config);

    let cors = CorsLayer::new()
        .allow_origin(config.frontend_origin.parse::<axum::http::HeaderValue>()?)
        .allow_credentials(true)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::PATCH, Method::DELETE])
        .allow_headers([CONTENT_TYPE, AUTHORIZATION]);

    let backend_port = config.backend_port;

    // GovernorLayer needs a 'static config; leaking it is the crate's documented pattern
    // since the layer lives for the process lifetime anyway.
    //
    // Everything arrives via the nginx reverse proxy, so the TCP peer the default
    // extractor would key on is always nginx's container IP — every real client
    // shares one bucket. SmartIpKeyExtractor reads X-Forwarded-For/X-Real-IP
    // (both set by nginx/conf.d/default.conf) to key per actual client instead.
    //
    // Two separate buckets, not one shared across the whole API: a single page
    // load already fires half a dozen concurrent requests (session refresh,
    // pages, workspaces, shared/favorite pages, page content, the collab WS
    // handshake), so a limit tight enough to stop credential/OTP brute-forcing
    // on /auth/login and /auth/register would throttle completely normal
    // browsing within a click or two. Login/register keep the strict bucket;
    // everything else (including /auth/refresh — it needs a valid session
    // already, so it isn't brute-forceable the same way) gets a much more
    // generous one.
    //
    // tower_governor's `.per_second(n)`/`.per_millisecond(n)` set the
    // replenish PERIOD (time between adding one token), not a request-rate —
    // `.per_second(20)` means one token every 20 SECONDS, i.e. ~0.05 req/s
    // sustained, not 20 req/s. Express an intended rate of R req/s as
    // `.per_millisecond(1000 / R)`.
    let strict_conf = Box::leak(Box::new(
        GovernorConfigBuilder::default()
            .key_extractor(SmartIpKeyExtractor)
            .per_millisecond(200) // ~5 req/s sustained
            .burst_size(20)
            .finish()
            .expect("valid governor rate-limit config"),
    ));
    let standard_conf = Box::leak(Box::new(
        GovernorConfigBuilder::default()
            .key_extractor(SmartIpKeyExtractor)
            .per_millisecond(50) // ~20 req/s sustained
            .burst_size(100)
            .finish()
            .expect("valid governor rate-limit config"),
    ));

    let state = AppState {
        db,
        redis,
        config: Arc::new(config),
        email: Arc::new(email),
        s3: Arc::new(s3),
        s3_presign: Arc::new(s3_presign),
        docs: collab::new_registry(),
        comments: comments::realtime::new_hub(),
    };

    let app = Router::new()
        .route("/health", get(health::health))
        .nest(
            "/auth",
            auth::sensitive_router()
                .layer(GovernorLayer { config: strict_conf })
                .merge(auth::router())
                .merge(users::router()),
        )
        .nest("/workspaces", workspaces::router())
        .merge(pages::router())
        .merge(sharing::router())
        .merge(collab::router())
        .merge(export::router())
        .merge(versions::router())
        .merge(comments::router())
        .merge(comments::realtime::router())
        .layer(GovernorLayer { config: standard_conf })
        .layer(CookieManagerLayer::new())
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], backend_port));
    tracing::info!("listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}

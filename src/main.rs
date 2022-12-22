use std::{collections::HashMap, convert::Infallible, error::Error, sync::Mutex};

use clap::Parser;
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server,
};
use once_cell::sync::Lazy;
use sqlx::{Any, AnyPool, Pool};
use tracing::{debug, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

static METRICS: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new(String::new()));

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about,
    long_about = "Turn your vaultwarden database into a api endpoint\ngithub: https://github.com/Tricked-dev/vwmetrics\nlicense: Apache-2.0"
)]
struct Cli {
    /// the database url to connect to `sqlite://db.sqlite3?mode=ro` for sqlite, `postgres://user:pass@localhost/db` for postgres or `mysql://user:pass@localhost/db` for mysql/mariadb
    #[clap(short, long, env)]
    database_url: String,
    /// the port to listen on
    #[clap(short, long, env, default_value = "3040")]
    port: u16,
    /// the host to bind to
    #[clap(short = 'b', long, env, default_value = "127.0.0.1")]
    host: String,
    /// Time between connecting and updating the metrics
    #[clap(short, long, env, default_value = "60")]
    update_seconds: u64,
}
/// Turn a Vaultwarden database into a metrics api endpoint.
async fn main_program() -> Result<(), Box<dyn Error + Send + Sync>> {
    let cli = Cli::parse();
    let db_url = match cli.database_url.clone() {
        url if url.starts_with("sqlite://") && !url.contains("?mode=ro") => {
            format!("{url}?mode=ro")
        }
        url if !url.contains("://") => {
            format!("sqlite://{url}?mode=ro")
        }
        url => url,
    };

    tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(std::time::Duration::from_secs(cli.update_seconds));
        loop {
            interval.tick().await;
            if let Err(e) = update_metrics(&db_url).await {
                tracing::error!("Error updating metrics: {}", e);
            }
        }
    });

    let addr = format!("{}:{}", cli.host, cli.port).parse()?;

    let make_service = make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle)) });

    let server = Server::bind(&addr).serve(make_service);

    if let Err(e) = server.await {
        eprintln!("server error: {e}");
    }

    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "error,vwmetrics=warn,debug,info,sqlx=warn".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // doing it this way gives better development errors
    main_program().await
}

async fn handle(req: Request<hyper::Body>) -> Result<Response<Body>, Infallible> {
    info!(
        target: "request",
        method = ?req.method(),
        uri = ?req.uri(),
        user_agent = ?req.headers().get("user-agent"),
    );

    Ok(Response::new(Body::from(METRICS.lock().unwrap().clone())))
}

async fn update_metrics(db_url: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    // reconnect every time to make sqlite work when the database has been overwritten by another program.
    let pool = AnyPool::connect(db_url).await?;

    let data = get_data(&pool).await?;

    let mut metrics = String::new();
    for (key, value) in data {
        metrics.push_str(&prometheus_stat(
            &format!("The number of {key}"),
            &format!("vaultwarden_{key}_count"),
            value,
        ));
    }

    *METRICS.lock().unwrap() = metrics;
    Ok(())
}

pub fn prometheus_stat<T>(help: &str, name: &str, value: T) -> String
where
    T: std::fmt::Display,
{
    format!("# HELP {name} {help}\n# TYPE {name} gauge\n{name} {value}\n")
}
type CountInt = i32;
async fn get_data(
    pool: &Pool<Any>,
) -> Result<HashMap<String, CountInt>, Box<dyn Error + Send + Sync>> {
    let mut res = HashMap::new();

    let mut tx = pool.begin().await?;
    debug!("Getting data from database");
    macro_rules! method_new {
        ($($ret:ident),*) => {
            $(
                // does not work without casting on postgresql
                res.insert(
                    stringify!($ret).to_string(),
                    sqlx::query_as::<_, (CountInt,)>(stringify!(SELECT CAST(count(*) as integer) FROM $ret))
                        .fetch_one(&mut tx)
                        .await?
                        .0,
                );
            )*
        };
    }

    method_new!(
        attachments,
        ciphers,
        ciphers_collections,
        collections,
        devices,
        emergency_access,
        favorites,
        folders,
        folders_ciphers,
        invitations,
        org_policies,
        organizations,
        sends,
        twofactor,
        twofactor_incomplete,
        users,
        users_collections,
        users_organizations
    );
    tx.commit().await?;
    debug!("Got data from database");
    Ok(res)
}

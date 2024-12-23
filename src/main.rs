use std::{collections::HashMap, sync::Mutex};

use anyhow::Result;
use clap::Parser;
use code::http1_server;
use once_cell::sync::Lazy;
use sqlx::{Any, AnyPool, Pool};
use tokio::task;
use tracing::debug;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod code;
pub mod support;

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

#[tokio::main(flavor = "current_thread")]
#[allow(clippy::needless_return)]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "error,vwmetrics=warn,debug,info,sqlx=warn".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

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

    sqlx::any::install_default_drivers();
    let local = task::LocalSet::new();

    local.spawn_local(async move {
        let mut interval =
            tokio::time::interval(std::time::Duration::from_secs(cli.update_seconds));
        loop {
            interval.tick().await;
            if let Err(e) = update_metrics(&db_url).await {
                tracing::error!("Error updating metrics: {}", e);
                panic!("Exiting program due to panic!")
            }
        }
    });

    let addr = format!("{}:{}", cli.host, cli.port).parse()?;
    local
        .run_until(async move {
            let output = http1_server(addr).await;
            if let Err(e) = output {
                tracing::error!("Error in http server: {}", e);
                panic!("Exiting program due to panic!")
            }
        })
        .await;
    Ok(())
}

async fn update_metrics(db_url: &str) -> Result<()> {
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
async fn get_data(pool: &Pool<Any>) -> Result<HashMap<String, CountInt>, anyhow::Error> {
    let mut res = HashMap::new();

    debug!("Getting data from database");
    macro_rules! method_new {
        ($($ret:ident),*) => {
            $(
                // does not work without casting on postgresql
                res.insert(
                    stringify!($ret).to_string(),
                    sqlx::query_as::<_, (CountInt,)>(stringify!(SELECT CAST(count(*) as integer) FROM $ret))
                        .fetch_one(pool)
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
    debug!("Got data from database");
    Ok(res)
}

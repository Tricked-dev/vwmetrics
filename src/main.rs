use std::{collections::HashMap, convert::Infallible, error::Error, sync::Mutex, thread};

use hyper::{
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server,
};
use once_cell::sync::Lazy;
use rusqlite::Connection;

static METRICS: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new(String::new()));

/// turn a vaulwarden database into a metrics api endpoint
async fn main_program() -> Result<(), Box<dyn Error + Send + Sync>> {
    tokio::spawn(async {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            thread::spawn(update_metrics);
        }
    });

    let addr = "127.0.0.1:3040".parse()?;

    let make_service = make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle)) });

    // Then bind and serve...
    let server = Server::bind(&addr).serve(make_service);

    // And run forever...
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    tokio::runtime::Builder::new_current_thread()
        .max_blocking_threads(1)
        .enable_all()
        .build()?
        .block_on(main_program())
}

async fn handle(_req: Request<hyper::Body>) -> Result<Response<Body>, Infallible> {
    Ok(Response::new(Body::from(METRICS.lock().unwrap().clone())))
}

fn update_metrics() -> Result<(), Box<dyn Error + Send + Sync>> {
    let conn = rusqlite::Connection::open("db.sqlite3")?;
    let data = get_data(&conn)?;
    let mut metrics = String::new();
    for (key, value) in data {
        metrics.push_str(&prometheus_stat(
            &format!("The number of {key}"),
            &format!("vaultwarden_{key}_count", key = key),
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
    format!("# HELP {name} {help}\n# TYPE {name} guage\n{name} {value}\n")
}

fn get_data(conn: &Connection) -> Result<HashMap<String, usize>, Box<dyn Error + Send + Sync>> {
    let mut res = HashMap::new();
    macro_rules! method_new {
        ($ret:ident) => {
            let $ret = conn.query_row(stringify!(SELECT count(*) FROM $ret), (), |row| row.get(0))?;
            res.insert(stringify!($ret).to_string(), $ret);
        };
    }
    method_new!(attachments);
    method_new!(ciphers);
    method_new!(ciphers_collections);
    method_new!(collections);
    method_new!(devices);
    method_new!(emergency_access);
    method_new!(favorites);
    method_new!(folders);
    method_new!(folders_ciphers);
    method_new!(invitations);
    method_new!(org_policies);
    method_new!(organizations);
    method_new!(sends);
    method_new!(twofactor);
    method_new!(twofactor_incomplete);
    method_new!(users);
    method_new!(users_collections);
    method_new!(users_organizations);

    Ok(res)
}

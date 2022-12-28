# VWMetrics

Turn your Vaultwarden database into Prometheus metrics.

## Usage

Build the binary from source or download the arm build from the releases page.

```
Turn your vaultwarden database into a api endpoint
github: https://github.com/Tricked-dev/vwmetrics
license: Apache-2.0

Usage: vwmetrics [OPTIONS] --database-url <DATABASE_URL>

Options:
  -d, --database-url <DATABASE_URL>
          the database url to connect to `sqlite://db.sqlite3?mode=ro` for sqlite, `postgres://user:pass@localhost/db` for postgres or `mysql://user:pass@localhost/db` for mysql/mariadb

          [env: DATABASE_URL=]

  -p, --port <PORT>
          the port to listen on

          [env: PORT=]
          [default: 3040]

  -b, --host <HOST>
          the host to bind to

          [env: HOST=]
          [default: 127.0.0.1]

  -u, --update-seconds <UPDATE_SECONDS>
          Time between connecting and updating the metrics

          [env: UPDATE_SECONDS=]
          [default: 60]

  -h, --help
          Print help information (use `-h` for a summary)

  -V, --version
          Print version information
```

The metrics endpoint gets started on `127.0.0.1:3040/metrics` by default.

### systemd

```init
# /home/<user>/.config/systemd/user/vwmetrics.service
[Unit]
Description=vwmetrics 
After=network.target

[Service]
ExecStart=/home/<user>/vwmetrics/vwmetrics
Environment="UPDATE_SECS=7200"
Environment="DATABASE_UR=query string"
Type=simple
Restart=always
WorkingDirectory=/home/<user>/vwmetrics

[Install]
WantedBy=default.target
RequiredBy=network.target
```

## Example output

![](.github/pics/pic.png)

[![](.github/pics/prev.png)](./.github/dash.json)

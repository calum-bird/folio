# FolioFS

A network drive that surfaces cloud data as Markdown so AI agents can read and
write it with normal file tools.

The architecture has two pieces, both Rust:

1. A WebDAV server (`dav-server` crate) that exposes a directory of rendered
   Markdown files.
2. A client that runs locally as an auth-terminating WebDAV proxy. It uses
   Clerk Authorization Code + PKCE for user login, speaks dumb Basic auth to
   `127.0.0.1`, then asks the OS WebDAV client to mount that localhost
   endpoint. The OS only ever sees harmless localhost credentials; Clerk tokens
   stay in the OS keychain and this process.

A separate render process (later) syncs cloud sources and writes Markdown into
the directory the WebDAV server serves.

## Status

Prototype, macOS only:

- `dav-server/` Rust WebDAV server. Backed by `dav_server::localfs::LocalFs`
  pointed at `render/`. Uses `FakeLs` so OS clients are happy with their LOCK
  probes.
- `client/` Rust local proxy and auto-mount supervisor:
  - Uses Clerk as IdP with Authorization Code + PKCE. The binary carries no
    client secret.
  - Opens the system browser for login, receives the callback on an ephemeral
    loopback port, then stores access and refresh tokens in the OS keychain.
  - Refreshes access tokens at 80% TTL and reactively on a 401 from upstream.
  - Runs an axum reverse proxy on `127.0.0.1` that strips Basic, injects
    Bearer, and forwards the request verbatim.
  - Drives `osascript` to mount the localhost URL; discovers the mount path
    by diffing `/Volumes/` before and after.
  - Unmounts cleanly on Ctrl-C.
  - Optional tray mode (`--features tray -- --tray`) adds a menu bar icon with
    auth/proxy/mount status plus open, mount, unmount, and quit actions.
- `render/` hand-written sample Markdown tree. Stand-in for the eventual
  render output.

Not built yet:

- Linux/Windows mount adapters.
- On-demand `DavFileSystem` impl that renders from a JSON cache instead of
  reading from disk.
- OS auto-mount installers (LaunchAgent, scheduled task, systemd user unit).
- Reconnect UX.
- Streaming request bodies through the proxy. Bodies are currently buffered
  to 64 MiB to enable the 401-then-retry path.

## Layout

```
foliofs/
  Cargo.toml          workspace
  dav-server/         WebDAV endpoint (LocalFs now, DavFileSystem later)
  render/             sample Markdown tree, served by dav-server
  client/             local auth-terminating proxy and auto-mount
```

## Build

Native:

```sh
cargo build --release
```

Tray client:

```sh
cargo build -p foliofs-client --release --features tray
```

Cross targets (after `rustup target add <triple>`):

```sh
cargo build --release --target aarch64-apple-darwin
cargo build --release --target x86_64-apple-darwin
cargo build --release --target aarch64-unknown-linux-gnu
cargo build --release --target x86_64-unknown-linux-gnu
```

## Run the prototype (local, no auth)

Terminal 1:

```sh
cargo run -p foliofs-dav-server -- --no-auth --bind 127.0.0.1:4918 --root render
```

Terminal 2:

```sh
cargo run -p foliofs-client -- --no-auth --upstream http://127.0.0.1:4918
```

The client picks a random localhost port, mounts it via the OS, and logs the
discovered mount path. Open it in Finder, then Ctrl-C the client to unmount.

Tray mode:

```sh
cargo run -p foliofs-client --features tray -- --tray --no-auth --upstream http://127.0.0.1:4918
```

The tray menu shows whether auth is local no-auth or logged in, the localhost
proxy address, and the mount state. You can mount and unmount without quitting;
Quit still performs a clean unmount first.

## Run with Clerk auth

The client is configured as a public native client:

- Clerk authorize URL: `https://settled-hamster-79.clerk.accounts.dev/oauth/authorize`
- Clerk token URL: `https://settled-hamster-79.clerk.accounts.dev/oauth/token`
- Clerk frontend user URL: `https://settled-hamster-79.clerk.accounts.dev/v1/me`
- Client ID: `rjHHgXHHq5Qhkqld`
- Redirect shape: `http://127.0.0.1:<ephemeral>/callback`

Run the client without `--no-auth`:

```sh
cargo run -p foliofs-client -- --upstream https://api.folio.fs
```

On first run, FolioFS opens the system browser for Clerk login. It stores
`access_token` and `refresh_token` in the OS keychain under the `foliofs`
service. Later runs reuse or refresh those tokens. If the refresh token has
expired, browser login runs again. The JWT currently only provides `sub`, so
the client calls Clerk's frontend `/v1/me` endpoint to show the user's name or
email in the tray.

To test Clerk auth against the local WebDAV prototype, run the local server and
omit `--no-auth`:

```sh
cargo run -p foliofs-dav-server -- --bind 127.0.0.1:4918 --root render
cargo run -p foliofs-client --features tray -- --tray --upstream http://127.0.0.1:4918
```

The proxy injects the Clerk bearer token and the local WebDAV server validates
it against Clerk's JWKS before serving the request. Server logs include the
verified Clerk `sub`. Requests are mapped into `render/<sub>/...`, so each user
gets a separate local subtree. For one-off local debugging, add `--log-raw-jwt`
to the server command to log the full token.

## Deploy the WebDAV server on AWS

Terraform for the lightweight AWS backend lives in `infra/aws`. It creates an
ECR repository, ALB, ECS cluster, ECS Managed Instances capacity provider, S3
bucket, S3 Files file system, mount targets, IAM roles, and the ECS service.
The server still sees a plain local directory at `/data`.

The web app adds a connector control plane on top of that storage:

- DynamoDB stores connection metadata and user-facing sync status keyed by
  Clerk `sub`.
- Secrets Manager stores provider OAuth tokens, with one secret per connection.
- The web app is responsible for OAuth connect/disconnect and browsing data.
- EventBridge Scheduler invokes a dispatcher Lambda, which sends due sync jobs
  to SQS.
- An SQS-triggered worker Lambda reads provider tokens from Secrets Manager,
  renders Markdown through the shared `connectors` crate, and writes to S3 Files
  mounted at `/mnt/folio`.

The web app needs these environment variables when connection management is
enabled:

```sh
FOLIO_CONNECTIONS_TABLE=foliofs-connections
FOLIO_CONNECTION_SECRET_PREFIX=foliofs/connections
FOLIO_CONNECTION_SECRETS_KMS_KEY_ID=<kms-key-arn>
FOLIO_SYNC_INTERVAL_SECONDS=3600
GITHUB_OAUTH_CLIENT_ID=<github-oauth-client-id>
GITHUB_OAUTH_CLIENT_SECRET=<github-oauth-client-secret>
```

Bootstrap the ECR repository first:

```sh
cd infra/aws
terraform init
terraform apply -target=aws_ecr_repository.app
```

Build and push the ARM container image:

```sh
AWS_REGION=us-east-1
ECR_REPO="$(aws ecr describe-repositories --repository-names foliofs --query 'repositories[0].repositoryUri' --output text)"

aws ecr get-login-password --region "$AWS_REGION" \
  | docker login --username AWS --password-stdin "${ECR_REPO%/*}"

cd ../..
docker buildx build --platform linux/arm64 -t "$ECR_REPO:latest" --push .
```

Build and push the sync Lambda images:

```sh
DISPATCHER_REPO="$(aws ecr describe-repositories --repository-names foliofs-sync-dispatcher --query 'repositories[0].repositoryUri' --output text)"
WORKER_REPO="$(aws ecr describe-repositories --repository-names foliofs-sync-worker --query 'repositories[0].repositoryUri' --output text)"

docker buildx build --platform linux/arm64 -f Dockerfile.sync-dispatcher -t "$DISPATCHER_REPO:latest" --push .
docker buildx build --platform linux/arm64 -f Dockerfile.sync-worker -t "$WORKER_REPO:latest" --push .
```

Apply the rest of the stack:

```sh
cd infra/aws
terraform apply \
  -var "container_image=$ECR_REPO:latest" \
  -var "sync_dispatcher_image=$DISPATCHER_REPO:latest" \
  -var "sync_worker_image=$WORKER_REPO:latest"
```

By default this runs one task on ECS Managed Instances using `t4g.medium`-class
ARM capacity and mounts S3 Files at `/data` for `LocalFS`. Scale by raising
`desired_count`; the service uses one host-networked task per managed instance
for the WebDAV server. The Lambda sync worker runs in private subnets, mounts S3
Files at `/mnt/folio`, and uses NAT egress for provider APIs like GitHub.

## License

MIT OR Apache-2.0

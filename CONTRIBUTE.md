# Contributing to FolioFS

This document is for local development, releases, and backend deployment. For
user-facing installation and usage, see [README.md](README.md).

## Repository Layout

```text
folio/
  client/             macOS CLI, tray app, local WebDAV proxy
  dav-server/         hosted WebDAV server
  connectors/         provider renderers for Markdown output
  sync-dispatcher/    scheduled Lambda that enqueues sync work
  sync-worker/        SQS Lambda that renders provider data into storage
  www.foliofs.dev/    Next.js web app and install.sh host
  infra/aws/          Terraform for the hosted backend
  scripts/            deploy and installer scripts
  render/             local sample Markdown tree
```

## Build

Build everything:

```sh
cargo build
```

Build the tray-enabled macOS CLI:

```sh
cargo build -p foliofs-client --features tray
./target/debug/folio --version
```

Build the Apple Silicon release target:

```sh
rustup target add aarch64-apple-darwin
cargo build -p foliofs-client --release --features tray --target aarch64-apple-darwin
```

## Local WebDAV Server

Run the local server without auth:

```sh
cargo run -p foliofs-dav-server -- --no-auth --bind 127.0.0.1:4918 --root render
```

Mount it in the foreground:

```sh
cargo run -p foliofs-client --features tray -- mount --no-auth --upstream http://127.0.0.1:4918
```

Run the tray in the foreground:

```sh
cargo run -p foliofs-client --features tray -- tray --no-auth --upstream http://127.0.0.1:4918
```

Run the tray detached with a per-user LaunchAgent:

```sh
cargo run -p foliofs-client --features tray -- start --no-auth --upstream http://127.0.0.1:4918
cargo run -p foliofs-client --features tray -- status
cargo run -p foliofs-client --features tray -- stop
```

## Clerk Auth

The native client uses Clerk Authorization Code + PKCE.

- Authorize URL:
  `https://settled-hamster-79.clerk.accounts.dev/oauth/authorize`
- Token URL: `https://settled-hamster-79.clerk.accounts.dev/oauth/token`
- Frontend user URL: `https://settled-hamster-79.clerk.accounts.dev/v1/me`
- Client ID: `rjHHgXHHq5Qhkqld`
- Redirect shape: `http://127.0.0.1:<ephemeral>/callback`

Login stores `access_token` and `refresh_token` in macOS Keychain under the
`foliofs` service:

```sh
cargo run -p foliofs-client --features tray -- login
cargo run -p foliofs-client --features tray -- whoami
cargo run -p foliofs-client --features tray -- start
```

Use `folio login --force` to force a new browser flow. Use `folio logout` to
stop the client, unmount the volume, and clear Keychain tokens.

To test Clerk auth against the local WebDAV server, omit `--no-auth` from both
commands:

```sh
cargo run -p foliofs-dav-server -- --bind 127.0.0.1:4918 --root render
cargo run -p foliofs-client --features tray -- tray --upstream http://127.0.0.1:4918
```

Server logs include the verified Clerk `sub`. Requests map to
`render/<sub>/...`, so each user gets a separate local subtree. For one-off
debugging, add `--log-raw-jwt` to the server command.

## Website

Run the Next.js app:

```sh
cd www.foliofs.dev
npm run dev
```

The public installer and uninstaller are served from:

```text
www.foliofs.dev/public/install.sh
www.foliofs.dev/public/uninstall.sh
```

When deployed, this makes the user install command work:

```sh
curl -fsSL https://foliofs.dev/install.sh | sh
```

Keep install/uninstall changes in `www.foliofs.dev/public/`; these are the
canonical public scripts served by the website.

## Release the CLI

The release workflow is `.github/workflows/release-client.yml`. It builds only
the Apple Silicon target:

```text
folio-aarch64-apple-darwin.tar.gz
folio-aarch64-apple-darwin.tar.gz.sha256
```

Publish a release by pushing a tag:

```sh
git tag v0.1.0
git push origin v0.1.0
```

The installer downloads the latest release by default. For a specific version:

```sh
FOLIO_VERSION=v0.1.0 sh www.foliofs.dev/public/install.sh
```

Public distribution should add Developer ID code signing and notarization before
the release is considered production-grade.

## Deploy the Backend

Terraform for the AWS backend lives in `infra/aws`. It creates ECR, ALB, ECS,
S3 Files, DynamoDB, KMS, SQS, Lambda, EventBridge Scheduler, and related IAM.

The main deploy script builds and pushes the WebDAV server, sync dispatcher, and
sync worker images, applies Terraform, and force-rolls ECS:

```sh
AWS_PROFILE=calum AWS_REGION=us-west-2 bash scripts/deploy-backend.sh
```

The web app needs these environment variables when connection management is
enabled:

```sh
FOLIO_CONNECTIONS_TABLE=foliofs-connections
FOLIO_CONNECTION_SECRETS_KMS_KEY_ID=<kms-key-arn>
FOLIO_SYNC_QUEUE_URL=<sqs-url>
GITHUB_OAUTH_CLIENT_ID=<github-oauth-client-id>
GITHUB_OAUTH_CLIENT_SECRET=<github-oauth-client-secret>
LINEAR_OAUTH_CLIENT_ID=<linear-oauth-client-id>
LINEAR_OAUTH_CLIENT_SECRET=<linear-oauth-client-secret>
SLACK_OAUTH_CLIENT_ID=<slack-oauth-client-id>
SLACK_OAUTH_CLIENT_SECRET=<slack-oauth-client-secret>
```

## Checks

Run focused Rust checks before sending changes:

```sh
cargo check -p foliofs-client --features tray
cargo clippy -p foliofs-client --features tray
cargo check -p foliofs-dav-server
```

For the website:

```sh
cd www.foliofs.dev
npm run lint
```

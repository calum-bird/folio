# FolioFS

Mount your cloud services as a read-only Markdown drive for local tools and AI
agents.

FolioFS turns connected services like Linear, GitHub, and Slack into a normal
macOS network volume. Once mounted, your agent can use the file operations it
already knows:

```sh
ls /Volumes/foliofs.dev
cat /Volumes/foliofs.dev/linear/issues/*.md
grep -R "launch" /Volumes/foliofs.dev
```

The local drive is read-only by design. Cloud data is rendered upstream, mounted
over WebDAV, and exposed locally through an auth-terminating proxy that keeps
OAuth tokens in macOS Keychain.

## Install

FolioFS currently ships for Apple Silicon Macs.

```sh
curl -fsSL https://foliofs.dev/install.sh | sh
```

## Quick Start

Log in once:

```sh
folio login
```

Start the menu-bar app:

```sh
folio start
```

Open the mounted drive:

```sh
open /Volumes/foliofs.dev
```

## CLI

```sh
folio login      # browser login, store access/refresh tokens in Keychain
folio whoami     # print the current account
folio start      # start the menu-bar app detached with launchd
folio status     # print LaunchAgent and mount status
folio stop       # stop the menu-bar app and unmount /Volumes/foliofs.dev
folio logout     # stop FolioFS and clear Keychain tokens
folio tray       # run the menu-bar app in the foreground for debugging
folio mount      # foreground mount flow until Ctrl-C
```

Defaults are production-ready:

```sh
folio start
# equivalent to:
folio start --upstream https://api.foliofs.dev --mount-name foliofs.dev
```

## How It Works

FolioFS has three pieces:

- The hosted WebDAV service serves a per-user rendered Markdown tree from
  FolioFS storage.
- The local `folio` client performs Clerk Authorization Code + PKCE login,
  stores tokens in macOS Keychain, and runs a localhost WebDAV proxy.
- macOS mounts the localhost proxy as `/Volumes/foliofs.dev`; the OS sees only
  short-lived local Basic credentials while the proxy injects upstream bearer
  tokens.

The upstream server advertises WebDAV class 1 and only allows read methods, so
macOS mounts the volume read-only and write attempts fail locally.

## Requirements

- macOS on Apple Silicon
- A FolioFS account at [foliofs.dev](https://foliofs.dev)
- Connected cloud integrations in the web app

## Development

Contributor setup, local server commands, release steps, and AWS deployment
notes live in [CONTRIBUTE.md](CONTRIBUTE.md).

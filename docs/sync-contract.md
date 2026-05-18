# FolioFS Sync Contract

The sync worker Lambda renders third-party SaaS data into the same S3 Files tree
that the WebDAV server exposes. The DAV server scopes each request to the Clerk
`sub`, so sync output must write to the matching sanitized user directory under
the Lambda mount:

```text
/mnt/folio/<sanitized_clerk_sub>/<provider>/
```

The sanitizer must match `dav-server/src/auth.rs`: ASCII letters, numbers,
hyphen, and underscore are preserved; every other character becomes `_`.

## Connection State

Connection metadata lives in DynamoDB. Refresh/access tokens live in Secrets
Manager and are referenced by ARN from the connection record. The worker must
obtain a DynamoDB lease before reading provider tokens or writing files.

EventBridge Scheduler invokes the sync dispatcher on a fixed cadence. The
dispatcher queries DynamoDB for due connections and sends one SQS job per
connection. The SQS-triggered worker handles exactly one connection per job.

## Rendered Files

Each connector owns its full connection subtree and must render deterministic
Markdown paths. Files should include YAML-style frontmatter with at least:

- `provider`
- `kind`
- stable provider id fields
- provider URL when available
- source timestamps when available

Human-readable Markdown follows the frontmatter. Renderers should prefer stable
filenames derived from provider ids or slugs, not display titles alone.

Each connector module owns both its sync code and templates. For example:

```text
connectors/src/github/connector.rs
connectors/src/github/templates/
```

Connectors implement a shared interface with a lightweight `plan` step and a
full `sync` step. The `plan` step reports changed entity counts and cursors so
future dispatch logic can avoid full downloads when a provider supports
incremental APIs.

## Atomic Writes

The worker writes a full replacement tree to:

```text
/mnt/folio/<user>/<provider>.next-<pid>-<timestamp>
```

After all files are written, it renames the existing tree aside, renames the
new tree into place, and removes the old tree. This prevents DAV clients from
seeing partially written sync output.

## Deletions

For the initial implementation, a full-tree replacement is the deletion model:
objects missing from the latest provider response disappear from the rendered
tree. Connectors that need auditability can add tombstone markdown files later,
but the default user-facing folio should reflect current provider state.

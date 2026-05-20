import { auth } from "@clerk/nextjs/server";
import Link from "next/link";

import {
  PageShell,
  SectionHeading,
  SiteHeader,
} from "@/app/_components/site-chrome";
import { CONNECTORS } from "@/lib/connections/registry";
import { type ConnectionRecord } from "@/lib/connections/model";
import { listUserConnections } from "@/lib/connections/store";

const PROVIDER_HOSTS: Record<string, string> = {
  github: "github.com",
  slack: "slack.com",
  linear: "linear.app",
};

export async function ConnectionsDashboard() {
  const { userId } = await auth();
  if (!userId) {
    return null;
  }

  const connections = await loadConnections(userId);
  const liveProviders = new Set(connections.map((c) => c.provider));

  return (
    <PageShell>
      <SiteHeader />

      <main className="flex flex-1 flex-col pt-4 pb-16 sm:pt-6">
        <ActiveConnections connections={connections} />
        <AvailableConnectors live={liveProviders} />
      </main>
    </PageShell>
  );
}

async function loadConnections(userId: string): Promise<ConnectionRecord[]> {
  try {
    return await listUserConnections(userId);
  } catch {
    return [];
  }
}

function ActiveConnections({ connections }: { connections: ConnectionRecord[] }) {
  return (
    <section
      className="folio-fade-up"
      style={{ animationDelay: "80ms" }}
    >
      <SectionHeading
        id="active"
        meta={
          connections.length === 0
            ? "none yet"
            : `${connections.length} active`
        }
      >
        active
      </SectionHeading>

      {connections.length === 0 ? (
        <EmptyActiveBlock />
      ) : (
        <div className="folio-tree">
          <p className="folio-tree__root">~/connections/</p>
          <ul className="folio-tree__list grid grid-cols-1 gap-3 md:gap-4">
            {connections.map((connection, index) => (
              <ActiveRow
                key={connection.connectionId}
                connection={connection}
                isLast={index === connections.length - 1}
              />
            ))}
          </ul>
        </div>
      )}
    </section>
  );
}

function ActiveRow({
  connection,
  isLast,
}: {
  connection: ConnectionRecord;
  isLast: boolean;
}) {
  const branch = isLast ? "└──" : "├──";
  const host = PROVIDER_HOSTS[connection.provider] ?? connection.provider;
  const hasError = connection.status === "failed" || connection.lastSyncError;

  return (
    <li className="folio-card">
      <div className="folio-card__head">
        <h3 className="folio-card__title">
          <span
            aria-hidden
            className="folio-tree__branch"
            style={{ marginRight: "0.4em" }}
          >
            {branch}
          </span>
          <span style={{ color: "var(--vermillion)" }}>{host}</span>
          <span style={{ color: "var(--ink-faint)" }}>/</span>
          <span>{connection.providerAccountLogin}</span>
        </h3>
        <span className="folio-card__meta">
          <StatusDot status={connection.status} hasError={!!hasError} />
          {connection.status}
        </span>
      </div>

      <p className="folio-card__body">
        {connection.displayName !== connection.providerAccountLogin ? (
          <>
            <span style={{ color: "var(--ink)" }}>{connection.displayName}</span>
            <span style={{ color: "var(--ink-faint)" }}> · </span>
          </>
        ) : null}
        <span>last sync {formatRelative(connection.lastSyncFinishedAt)}</span>
        <span style={{ color: "var(--ink-faint)" }}> · </span>
        <span>next sync {formatRelative(connection.nextSyncAt)}</span>
      </p>

      {hasError ? (
        <p className="mt-2 text-[12px]" style={{ color: "var(--vermillion)" }}>
          {connection.lastSyncError ?? "sync failed"}
        </p>
      ) : null}

      <div className="mt-4 flex flex-wrap items-center gap-3 text-[11px] uppercase tracking-[0.16em]">
        <Link
          href={`/app?path=${encodeURIComponent(
            `/${connection.outputPrefix}`,
          )}`}
          className="folio-link"
        >
          open in browser
        </Link>
        <span style={{ color: "var(--ink-faint)" }}>·</span>
        <form
          action={`/api/connections/${connection.provider}/${connection.connectionId}/disconnect`}
          method="post"
        >
          <button type="submit" className="folio-link folio-link--muted">
            disconnect
          </button>
        </form>
      </div>
    </li>
  );
}

function EmptyActiveBlock() {
  return (
    <div
      className="border border-dashed p-8 text-center"
      style={{
        borderColor: "color-mix(in srgb, var(--ink) 30%, transparent)",
        color: "var(--ink-soft)",
      }}
    >
      <p className="folio-marginalia mb-2">nothing connected yet</p>
      <p className="text-[13px]">
        Pick a service below and we&rsquo;ll render its data into your folio.
      </p>
    </div>
  );
}

function AvailableConnectors({ live }: { live: Set<string> }) {
  return (
    <section
      className="folio-fade-up mt-12 sm:mt-16"
      style={{ animationDelay: "140ms" }}
    >
      <SectionHeading id="available" meta={`${CONNECTORS.length} ready to mount`}>
        available
      </SectionHeading>

      <div className="folio-tree">
        <p className="folio-tree__root">~/available/</p>
        <ul className="folio-tree__list">
          {CONNECTORS.map((connector, index) => {
            const isLast = index === CONNECTORS.length - 1;
            const isConnected = live.has(connector.provider);
            return (
              <li key={connector.provider} className="folio-tree__node">
                <Link
                  href={`/api/connections/${connector.provider}/start`}
                  className="folio-tree__row"
                  aria-label={`connect ${connector.displayName}`}
                >
                  <span aria-hidden className="folio-tree__branch">
                    {isLast ? "└──" : "├──"}{" "}
                  </span>
                  <span className="folio-tree__domain">
                    {PROVIDER_HOSTS[connector.provider] ?? connector.provider}
                  </span>
                  <span className="folio-tree__sub">
                    {" "}
                    — {connector.description.toLowerCase()}
                  </span>
                  {isConnected ? (
                    <span className="folio-tree__tag" style={{ color: "var(--vermillion)", borderColor: "var(--vermillion)" }}>
                      connected
                    </span>
                  ) : null}
                  <span aria-hidden className="folio-tree__arrow">
                    →
                  </span>
                </Link>
              </li>
            );
          })}
        </ul>
      </div>
    </section>
  );
}

function StatusDot({
  status,
  hasError,
}: {
  status: string;
  hasError: boolean;
}) {
  const color = hasError
    ? "var(--vermillion)"
    : status === "active"
      ? "var(--vermillion)"
      : status === "syncing"
        ? "var(--ink)"
        : "var(--ink-faint)";

  return (
    <span
      aria-hidden
      className="inline-block h-1.5 w-1.5 -translate-y-0.5"
      style={{ backgroundColor: color, marginRight: "0.55em" }}
    />
  );
}

function formatRelative(value: string | undefined) {
  if (!value) return "never";
  const target = new Date(value).getTime();
  const now = Date.now();
  const diffSeconds = Math.round((target - now) / 1000);
  const absSeconds = Math.abs(diffSeconds);
  const past = diffSeconds < 0;

  const formats: { unit: Intl.RelativeTimeFormatUnit; seconds: number }[] = [
    { unit: "year", seconds: 31536000 },
    { unit: "month", seconds: 2592000 },
    { unit: "week", seconds: 604800 },
    { unit: "day", seconds: 86400 },
    { unit: "hour", seconds: 3600 },
    { unit: "minute", seconds: 60 },
    { unit: "second", seconds: 1 },
  ];

  for (const { unit, seconds } of formats) {
    if (absSeconds >= seconds) {
      const count = Math.round(absSeconds / seconds);
      return past ? `${count} ${unit}${count === 1 ? "" : "s"} ago` : `in ${count} ${unit}${count === 1 ? "" : "s"}`;
    }
  }

  return past ? "just now" : "any moment";
}

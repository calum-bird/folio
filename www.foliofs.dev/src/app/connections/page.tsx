import { auth } from "@clerk/nextjs/server";
import Link from "next/link";

import { Button } from "@/components/ui/button";
import { CONNECTORS } from "@/lib/connections/registry";
import { listUserConnections } from "@/lib/connections/store";

export default async function ConnectionsPage() {
  const { userId } = await auth();
  if (!userId) {
    return null;
  }

  const connections = await loadConnections(userId);

  return (
    <main className="flex min-h-full flex-1 bg-zinc-950 text-zinc-100">
      <section className="mx-auto flex w-full max-w-5xl flex-col gap-6 p-6">
        <header className="flex items-center justify-between border-b border-zinc-800 pb-4">
          <div>
            <p className="text-xs uppercase tracking-[0.35em] text-zinc-500">FolioFS</p>
            <h1 className="mt-2 text-2xl font-semibold tracking-tight">Connections</h1>
          </div>
          <Button asChild variant="outline">
            <Link href="/">Back to folio</Link>
          </Button>
        </header>

        <section className="grid gap-4 md:grid-cols-3">
          {CONNECTORS.map((connector) => (
            <div className="border border-zinc-800 bg-zinc-900/40 p-5" key={connector.provider}>
              <h2 className="text-lg font-medium">{connector.displayName}</h2>
              <p className="mt-2 text-sm leading-6 text-zinc-400">{connector.description}</p>
              <Button asChild className="mt-4">
                <Link href={`/api/connections/${connector.provider}/start`}>
                  Connect {connector.displayName}
                </Link>
              </Button>
            </div>
          ))}
        </section>

        <section className="border border-zinc-800 bg-zinc-900/40">
          <div className="border-b border-zinc-800 px-4 py-3">
            <h2 className="text-sm font-medium uppercase tracking-[0.2em] text-zinc-500">
              Active connections
            </h2>
          </div>
          {connections.length > 0 ? (
            connections.map((connection) => (
              <div
                className="grid gap-3 border-b border-zinc-800 px-4 py-4 text-sm last:border-b-0 md:grid-cols-[1fr_12rem_10rem]"
                key={connection.connectionId}
              >
                <div>
                  <p className="font-medium">{connection.displayName}</p>
                  <p className="mt-1 text-zinc-500">
                    {connection.provider} / {connection.providerAccountLogin}
                  </p>
                  {connection.lastSyncError ? (
                    <p className="mt-2 text-red-300">{connection.lastSyncError}</p>
                  ) : null}
                </div>
                <div className="text-zinc-400">
                  <p>Status: {connection.status}</p>
                  <p>Last sync: {formatDate(connection.lastSyncFinishedAt)}</p>
                </div>
                <form action={`/api/connections/${connection.provider}/${connection.connectionId}/disconnect`} method="post">
                  <Button type="submit" variant="destructive">
                    Disconnect
                  </Button>
                </form>
              </div>
            ))
          ) : (
            <div className="px-4 py-10 text-center text-sm text-zinc-500">
              No cloud software is connected yet.
            </div>
          )}
        </section>
      </section>
    </main>
  );
}

async function loadConnections(userId: string) {
  try {
    return await listUserConnections(userId);
  } catch {
    return [];
  }
}

function formatDate(value: string | undefined) {
  if (!value) {
    return "never";
  }

  return new Intl.DateTimeFormat("en", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(value));
}

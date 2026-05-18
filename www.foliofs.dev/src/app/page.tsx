import { UserButton } from "@clerk/nextjs";
import Link from "next/link";

import {
  folioPathFromParam,
  listFolioDirectory,
  parentFolioPath,
  type FolioEntry,
} from "@/lib/folio-dav";

type HomeProps = {
  searchParams: Promise<{
    path?: string | string[];
  }>;
};

type DirectoryState =
  | { ok: true; entries: FolioEntry[] }
  | { ok: false; message: string };

export default async function Home({ searchParams }: HomeProps) {
  const params = await searchParams;
  const path = folioPathFromParam(params.path);
  const directory = await loadDirectory(path);

  return (
    <main className="flex min-h-full flex-1 bg-zinc-950 text-zinc-100">
      <section className="flex w-full flex-col">
        <header className="flex items-center justify-between border-b border-zinc-800 px-6 py-4">
          <div>
            <p className="text-xs uppercase tracking-[0.35em] text-zinc-500">FolioFS</p>
            <h1 className="mt-2 text-2xl font-semibold tracking-tight">Your folio</h1>
          </div>
          <div className="flex items-center gap-4">
            <Link className="text-sm text-zinc-300 hover:text-zinc-100" href="/connections">
              Connections
            </Link>
            <UserButton />
          </div>
        </header>

        <div className="flex flex-1 flex-col gap-6 p-6">
          <nav className="flex items-center gap-2 text-sm text-zinc-400">
            <Link className="text-zinc-100 hover:underline" href="/">
              folio
            </Link>
            {breadcrumbSegments(path).map((segment) => (
              <span className="flex items-center gap-2" key={segment.path}>
                <span>/</span>
                <Link className="text-zinc-100 hover:underline" href={browseHref(segment.path)}>
                  {segment.name}
                </Link>
              </span>
            ))}
          </nav>

          <div className="overflow-hidden border border-zinc-800 bg-zinc-900/40">
            <div className="grid grid-cols-[1fr_8rem_12rem] border-b border-zinc-800 px-4 py-2 text-xs uppercase tracking-[0.2em] text-zinc-500">
              <span>Name</span>
              <span>Size</span>
              <span>Modified</span>
            </div>

            {path !== "/" ? <ParentRow path={path} /> : null}
            {directory.ok ? <DirectoryRows entries={directory.entries} /> : <ErrorRow message={directory.message} />}
          </div>
        </div>
      </section>
    </main>
  );
}

async function loadDirectory(path: string): Promise<DirectoryState> {
  try {
    return { ok: true, entries: await listFolioDirectory(path) };
  } catch (error) {
    return {
      ok: false,
      message: error instanceof Error ? error.message : "Unable to load this folio directory.",
    };
  }
}

function DirectoryRows({ entries }: { entries: FolioEntry[] }) {
  if (entries.length === 0) {
    return (
      <div className="px-4 py-12 text-center text-sm text-zinc-500">
        This folder is empty.
      </div>
    );
  }

  return entries.map((entry) => <EntryRow entry={entry} key={entry.path} />);
}

function EntryRow({ entry }: { entry: FolioEntry }) {
  const href = entry.kind === "directory" ? browseHref(entry.path) : fileHref(entry.path);

  return (
    <Link
      className="grid grid-cols-[1fr_8rem_12rem] border-b border-zinc-800 px-4 py-3 text-sm last:border-b-0 hover:bg-zinc-800/60"
      href={href}
    >
      <span className="truncate">
        <span className="mr-2 text-zinc-500">{entry.kind === "directory" ? "dir" : "file"}</span>
        {entry.name}
      </span>
      <span className="text-zinc-400">{entry.kind === "directory" ? "--" : formatSize(entry.size)}</span>
      <span className="text-zinc-400">{formatDate(entry.lastModified)}</span>
    </Link>
  );
}

function ParentRow({ path }: { path: string }) {
  return (
    <Link
      className="grid grid-cols-[1fr_8rem_12rem] border-b border-zinc-800 px-4 py-3 text-sm hover:bg-zinc-800/60"
      href={browseHref(parentFolioPath(path))}
    >
      <span className="truncate text-zinc-300">..</span>
      <span />
      <span />
    </Link>
  );
}

function ErrorRow({ message }: { message: string }) {
  return (
    <div className="border-t border-red-500/20 bg-red-500/10 px-4 py-6 text-sm text-red-200">
      {message}
    </div>
  );
}

function breadcrumbSegments(path: string) {
  const segments = path.split("/").filter(Boolean);
  return segments.map((name, index) => ({
    name,
    path: `/${segments.slice(0, index + 1).join("/")}`,
  }));
}

function browseHref(path: string) {
  if (path === "/") {
    return "/";
  }

  return `/?path=${encodeURIComponent(path)}`;
}

function fileHref(path: string) {
  return `/api/folio/file?path=${encodeURIComponent(path)}`;
}

function formatSize(size: number) {
  if (size < 1024) {
    return `${size} B`;
  }

  const kilobytes = size / 1024;
  if (kilobytes < 1024) {
    return `${kilobytes.toFixed(1)} KB`;
  }

  return `${(kilobytes / 1024).toFixed(1)} MB`;
}

function formatDate(value: string | null) {
  if (!value) {
    return "--";
  }

  return new Intl.DateTimeFormat("en", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(value));
}

import Link from "next/link";

import {
  PageShell,
  PathDisplay,
  SiteHeader,
} from "@/app/_components/site-chrome";
import {
  folioPathFromParam,
  listFolioDirectory,
  parentFolioPath,
  type FolioEntry,
} from "@/lib/folio-dav";

type FolioBrowserProps = {
  pathParam: string | string[] | undefined;
};

type DirectoryState =
  | { ok: true; entries: FolioEntry[] }
  | { ok: false; message: string };

export async function FolioBrowser({ pathParam }: FolioBrowserProps) {
  const path = folioPathFromParam(pathParam);
  const directory = await loadDirectory(path);
  const isRoot = path === "/";

  return (
    <PageShell>
      <SiteHeader />

      <main className="flex flex-1 flex-col pt-4 pb-16 sm:pt-6">
        <div className="folio-fade-up">
          <PathDisplay
            segments={pathSegments(path)}
            size="sm"
          />
        </div>

        <div
          className="folio-fade-up mt-8"
          style={{ animationDelay: "100ms" }}
        >
          <div className="mb-4 flex items-baseline justify-end gap-4">
            <span
              className="folio-marginalia"
              style={{ letterSpacing: "0.06em", textTransform: "none" }}
            >
              {directory.ok
                ? entriesCountLabel(directory.entries.length, isRoot)
                : "error"}
            </span>
          </div>

          {directory.ok ? (
            <FileTree
              entries={directory.entries}
              currentPath={path}
              isRoot={isRoot}
            />
          ) : (
            <ErrorBlock message={directory.message} />
          )}
        </div>
      </main>
    </PageShell>
  );
}

async function loadDirectory(path: string): Promise<DirectoryState> {
  try {
    return { ok: true, entries: await listFolioDirectory(path) };
  } catch (error) {
    return {
      ok: false,
      message:
        error instanceof Error
          ? error.message
          : "Unable to load this folio directory.",
    };
  }
}

function FileTree({
  entries,
  currentPath,
  isRoot,
}: {
  entries: FolioEntry[];
  currentPath: string;
  isRoot: boolean;
}) {
  if (entries.length === 0 && isRoot) {
    return (
      <div className="folio-tree folio-tree--files">
        <EmptyState />
      </div>
    );
  }

  const rows: TreeRowDescriptor[] = [];
  if (!isRoot) {
    rows.push({
      kind: "parent",
      name: "..",
      href: browseHref(parentFolioPath(currentPath)),
    });
  }
  entries.forEach((entry) => {
    rows.push({
      kind: entry.kind,
      name: entry.name,
      href: entry.kind === "directory" ? browseHref(entry.path) : fileHref(entry.path),
      size: entry.size,
      lastModified: entry.lastModified,
    });
  });

  return (
    <div className="folio-tree folio-tree--files">
      {rows.length === 0 ? (
        <EmptyState />
      ) : (
        <ul className="folio-tree__list">
          {rows.map((row, index) => (
            <TreeRow
              key={`${row.kind}-${row.name}`}
              row={row}
              isLast={index === rows.length - 1}
            />
          ))}
        </ul>
      )}
    </div>
  );
}

type TreeRowDescriptor =
  | {
      kind: "parent";
      name: string;
      href: string;
    }
  | {
      kind: "directory" | "file";
      name: string;
      href: string;
      size: number;
      lastModified: string | null;
    };

function TreeRow({
  row,
  isLast,
}: {
  row: TreeRowDescriptor;
  isLast: boolean;
}) {
  const branch = isLast ? "└──" : "├──";
  const isParent = row.kind === "parent";

  return (
    <li className="folio-tree__node">
      <Link
        href={row.href}
        className="folio-tree__row"
        aria-label={
          isParent
            ? "go up one directory"
            : `${row.kind === "directory" ? "directory" : "file"}: ${row.name}`
        }
      >
        <span aria-hidden className="folio-tree__branch">
          {branch}{" "}
        </span>
        <span className="folio-tree__name">
          <span className="folio-tree__kind">
            {isParent
              ? ".."
              : row.kind === "directory"
                ? "dir"
                : "file"}
          </span>
          <span className="folio-tree__name-text">
            {isParent ? "" : row.name}
            {row.kind === "directory" ? (
              <span style={{ color: "var(--vermillion)" }}>/</span>
            ) : null}
          </span>
        </span>
        <span className="folio-tree__meta">
          {isParent || row.kind === "directory" ? "" : formatSize(row.size)}
        </span>
        <span className="folio-tree__meta">
          {isParent ? "" : formatDate(row.lastModified)}
        </span>
      </Link>
    </li>
  );
}

function EmptyState() {
  return (
    <div
      className="border-t border-dashed py-16 text-center"
      style={{
        borderColor: "color-mix(in srgb, var(--ink) 25%, transparent)",
        color: "var(--ink-faint)",
      }}
    >
      <p className="folio-marginalia mb-3">empty directory</p>
      <p className="text-[13px]" style={{ color: "var(--ink-soft)" }}>
        This directory has no files yet.
      </p>
    </div>
  );
}

function ErrorBlock({ message }: { message: string }) {
  return (
    <div
      className="border p-5 text-[13px]"
      style={{
        borderColor: "var(--vermillion)",
        color: "var(--vermillion)",
        backgroundColor: "color-mix(in srgb, var(--vermillion) 8%, transparent)",
      }}
    >
      <p className="folio-marginalia mb-2" style={{ color: "var(--vermillion)" }}>
        ¶ error
      </p>
      <p>{message}</p>
    </div>
  );
}

function pathSegments(path: string) {
  const segments = path.split("/").filter(Boolean);
  const out: { name: string; href?: string }[] = [
    { name: "mnt", href: browseHref("/") },
    { name: "foliofs.dev", href: browseHref("/") },
  ];

  segments.forEach((name, index) => {
    const targetPath = `/${segments.slice(0, index + 1).join("/")}`;
    out.push({ name, href: browseHref(targetPath) });
  });

  return out;
}

function entriesCountLabel(count: number, isRoot: boolean) {
  if (count === 0) {
    return isRoot ? "nothing mounted yet" : "empty";
  }
  if (count === 1) {
    return "1 entry";
  }
  return `${count} entries`;
}

function browseHref(path: string) {
  if (path === "/") return "/";
  return `/?path=${encodeURIComponent(path)}`;
}

function fileHref(path: string) {
  return `/api/folio/file?path=${encodeURIComponent(path)}`;
}

function formatSize(size: number) {
  if (size < 1024) return `${size} B`;
  const kilobytes = size / 1024;
  if (kilobytes < 1024) return `${kilobytes.toFixed(1)} KB`;
  return `${(kilobytes / 1024).toFixed(1)} MB`;
}

function formatDate(value: string | null) {
  if (!value) return "—";
  return new Intl.DateTimeFormat("en", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(value));
}

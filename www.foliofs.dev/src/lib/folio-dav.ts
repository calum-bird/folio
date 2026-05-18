import "server-only";

import { auth } from "@clerk/nextjs/server";
import { createClient, type FileStat } from "webdav";

const DEFAULT_DAV_URL = "http://127.0.0.1:4918";

export type FolioEntry = {
  name: string;
  path: string;
  kind: "file" | "directory";
  size: number;
  lastModified: string | null;
  mime: string | null;
};

export function folioPathFromParam(value: string | string[] | null | undefined) {
  if (Array.isArray(value)) {
    return normalizeFolioPath(value[0]);
  }

  return normalizeFolioPath(value);
}

export function parentFolioPath(path: string) {
  const normalized = normalizeFolioPath(path);
  if (normalized === "/") {
    return "/";
  }

  const parent = normalized.split("/").slice(0, -1).join("/");
  if (!parent) {
    return "/";
  }

  return parent;
}

export async function listFolioDirectory(path: string): Promise<FolioEntry[]> {
  const client = await createFolioClient();
  const contents = await client.getDirectoryContents(normalizeFolioPath(path));
  if (!Array.isArray(contents)) {
    return [];
  }

  return contents.map(toFolioEntry).sort(sortFolioEntries);
}

export async function getFolioFile(path: string) {
  const normalized = normalizeFolioPath(path);
  const client = await createFolioClient();
  const [stat, contents] = await Promise.all([
    client.stat(normalized),
    client.getFileContents(normalized, { format: "binary" }),
  ]);

  return {
    body: toResponseBody(contents),
    name: basename(normalized),
    mime: "mime" in stat && stat.mime ? stat.mime : "application/octet-stream",
  };
}

function normalizeFolioPath(value: string | null | undefined) {
  if (!value) {
    return "/";
  }

  const segments = value.split("/").filter(Boolean);
  const safeSegments = segments.filter((segment) => {
    if (segment === "." || segment === "..") {
      return false;
    }

    return true;
  });

  if (safeSegments.length === 0) {
    return "/";
  }

  return `/${safeSegments.join("/")}`;
}

async function createFolioClient() {
  const { getToken } = await auth();
  const token = await getToken();
  if (!token) {
    throw new Error("Missing Clerk session token");
  }

  return createClient(folioDavUrl(), {
    headers: {
      Authorization: `Bearer ${token}`,
    },
  });
}

function folioDavUrl() {
  return process.env.FOLIO_DAV_URL ?? DEFAULT_DAV_URL;
}

function toFolioEntry(stat: FileStat): FolioEntry {
  return {
    name: stat.basename || basename(stat.filename),
    path: normalizeFolioPath(stat.filename),
    kind: stat.type,
    size: stat.size,
    lastModified: stat.lastmod || null,
    mime: stat.mime ?? null,
  };
}

function sortFolioEntries(left: FolioEntry, right: FolioEntry) {
  if (left.kind !== right.kind) {
    return left.kind === "directory" ? -1 : 1;
  }

  return left.name.localeCompare(right.name);
}

function basename(path: string) {
  const segments = normalizeFolioPath(path).split("/").filter(Boolean);
  return segments.at(-1) ?? "folio";
}

function toResponseBody(contents: unknown) {
  if (contents instanceof ArrayBuffer) {
    return contents;
  }

  if (contents instanceof Uint8Array) {
    return contents;
  }

  return new TextEncoder().encode(String(contents));
}

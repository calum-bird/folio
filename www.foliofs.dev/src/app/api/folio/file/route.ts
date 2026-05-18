import { getFolioFile, folioPathFromParam } from "@/lib/folio-dav";

export async function GET(request: Request) {
  const url = new URL(request.url);
  const path = folioPathFromParam(url.searchParams.get("path"));
  const file = await getFolioFile(path);

  return new Response(file.body, {
    headers: {
      "content-disposition": `inline; filename="${escapeHeaderValue(file.name)}"`,
      "content-type": file.mime,
    },
  });
}

function escapeHeaderValue(value: string) {
  return value.replaceAll("\\", "\\\\").replaceAll('"', '\\"');
}

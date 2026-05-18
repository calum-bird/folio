import { auth } from "@clerk/nextjs/server";
import { cookies } from "next/headers";
import { NextResponse } from "next/server";

import {
  buildLinearConnection,
  buildLinearSecret,
  exchangeLinearCode,
  fetchLinearWorkspace,
} from "@/lib/connections/linear";
import { putConnectionSecret } from "@/lib/connections/secrets";
import { putConnection } from "@/lib/connections/store";

const STATE_COOKIE = "folio_linear_oauth_state";

export async function GET(request: Request) {
  const { userId } = await auth();
  if (!userId) {
    return new Response("Unauthorized", { status: 401 });
  }

  const url = new URL(request.url);
  const state = url.searchParams.get("state");
  const code = url.searchParams.get("code");
  const cookieStore = await cookies();
  const expectedState = cookieStore.get(STATE_COOKIE)?.value;
  cookieStore.delete(STATE_COOKIE);

  if (!state || state !== expectedState) {
    return redirectWithError(request, "Invalid OAuth state.");
  }

  if (!code) {
    return redirectWithError(request, "Linear did not return an authorization code.");
  }

  const token = await exchangeLinearCode(code, callbackUrl(request));
  const workspace = await fetchLinearWorkspace(token.access_token ?? "");
  const connectionId = `linear-${workspace.id}`;
  const secretArn = await putConnectionSecret(
    userId,
    "linear",
    connectionId,
    buildLinearSecret(workspace, token),
  );
  await putConnection(buildLinearConnection(userId, workspace, token, secretArn));

  return NextResponse.redirect(new URL("/connections?connected=linear", request.url));
}

function redirectWithError(request: Request, message: string) {
  const url = new URL("/connections", request.url);
  url.searchParams.set("error", message);
  return NextResponse.redirect(url);
}

function callbackUrl(request: Request) {
  const url = new URL(request.url);
  return `${url.origin}/api/connections/linear/callback`;
}

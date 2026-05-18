import { auth } from "@clerk/nextjs/server";
import { cookies } from "next/headers";
import { NextResponse } from "next/server";

import {
  buildSlackConnection,
  buildSlackSecret,
  exchangeSlackCode,
  fetchSlackWorkspace,
} from "@/lib/connections/slack";
import { putConnectionSecret } from "@/lib/connections/secrets";
import { putConnection } from "@/lib/connections/store";

const STATE_COOKIE = "folio_slack_oauth_state";

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
    return redirectWithError(request, "Slack did not return an authorization code.");
  }

  const token = await exchangeSlackCode(code, callbackUrl(request));
  const workspace = await fetchSlackWorkspace(token.access_token ?? "");
  const connectionId = `slack-${workspace.id}`;
  const secretArn = await putConnectionSecret(
    userId,
    "slack",
    connectionId,
    buildSlackSecret(workspace, token),
  );
  await putConnection(buildSlackConnection(userId, workspace, token, secretArn));

  return NextResponse.redirect(new URL("/connections?connected=slack", request.url));
}

function redirectWithError(request: Request, message: string) {
  const url = new URL("/connections", request.url);
  url.searchParams.set("error", message);
  return NextResponse.redirect(url);
}

function callbackUrl(request: Request) {
  const url = new URL(request.url);
  return `${url.origin}/api/connections/slack/callback`;
}

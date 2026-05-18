import { auth } from "@clerk/nextjs/server";
import { cookies } from "next/headers";
import { NextResponse } from "next/server";

import { githubAuthorizeUrl } from "@/lib/connections/github";

const STATE_COOKIE = "folio_github_oauth_state";

export async function GET(request: Request) {
  const { userId } = await auth();
  if (!userId) {
    return new Response("Unauthorized", { status: 401 });
  }

  const state = crypto.randomUUID();
  const redirectUri = callbackUrl(request);
  const cookieStore = await cookies();
  cookieStore.set(STATE_COOKIE, state, {
    httpOnly: true,
    maxAge: 10 * 60,
    path: "/",
    sameSite: "lax",
    secure: process.env.NODE_ENV === "production",
  });

  return NextResponse.redirect(githubAuthorizeUrl(state, redirectUri));
}

function callbackUrl(request: Request) {
  const url = new URL(request.url);
  return `${url.origin}/api/connections/github/callback`;
}

import "server-only";

import { cookies } from "next/headers";
import { NextResponse } from "next/server";

import { connectionPk, connectionSk } from "@/lib/connections/model";
import {
  type ConnectorDefinition,
  buildConnection,
  buildSecret,
  connectionIdFor,
  scopesForConnector,
} from "@/lib/connections/registry";
import { deleteConnection, getConnection, putConnection } from "@/lib/connections/store";
import { enqueueSync } from "@/lib/connections/sync";
import { encryptConnectionToken, tokenEncryptionContext } from "@/lib/connections/tokens";

export async function startOAuth(request: Request, connector: ConnectorDefinition) {
  const state = crypto.randomUUID();
  const cookieStore = await cookies();
  cookieStore.set(stateCookieName(connector), state, {
    httpOnly: true,
    maxAge: 10 * 60,
    path: "/",
    sameSite: "lax",
    secure: process.env.NODE_ENV === "production",
  });

  return NextResponse.redirect(
    connector.buildAuthorizeUrl({
      state,
      redirectUri: callbackUrl(request, connector),
      scopes: scopesForConnector(connector),
    }),
  );
}

export async function completeOAuth(
  request: Request,
  userId: string,
  connector: ConnectorDefinition,
) {
  const url = new URL(request.url);
  const state = url.searchParams.get("state");
  const code = url.searchParams.get("code");
  const cookieStore = await cookies();
  const expectedState = cookieStore.get(stateCookieName(connector))?.value;
  cookieStore.delete(stateCookieName(connector));

  if (!state || state !== expectedState) {
    return redirectWithError(request, "Invalid OAuth state.");
  }
  if (!code) {
    return redirectWithError(
      request,
      `${connector.displayName} did not return an authorization code.`,
    );
  }

  const token = await connector.exchangeCode({
    code,
    redirectUri: callbackUrl(request, connector),
  });
  const account = await connector.fetchAccount(token.accessToken);
  const connectionId = connectionIdFor(connector, account);
  const pk = connectionPk(userId);
  const sk = connectionSk(connector.provider, connectionId);
  const encryptedToken = await encryptConnectionToken(
    buildSecret(connector, account, token),
    tokenEncryptionContext(pk, sk),
  );

  const connection = buildConnection(connector, { userId, account, token, encryptedToken });
  await putConnection(connection);
  await enqueueSync(connection);

  return NextResponse.redirect(
    new URL(`/connections?connected=${connector.provider}`, request.url),
  );
}

export async function disconnectConnection(
  request: Request,
  userId: string,
  connector: ConnectorDefinition,
  connectionId: string,
) {
  const connection = await getConnection(userId, connector.provider, connectionId);
  if (!connection) {
    return NextResponse.redirect(new URL("/connections", request.url), 303);
  }

  await deleteConnection(userId, connector.provider, connectionId);

  return NextResponse.redirect(
    new URL(`/connections?disconnected=${connector.provider}`, request.url),
    303,
  );
}

function stateCookieName(connector: ConnectorDefinition) {
  return `folio_${connector.provider}_oauth_state`;
}

function callbackUrl(request: Request, connector: ConnectorDefinition) {
  const url = new URL(request.url);
  return `${url.origin}/api/connections/${connector.provider}/callback`;
}

function redirectWithError(request: Request, message: string) {
  const url = new URL("/connections", request.url);
  url.searchParams.set("error", message);
  return NextResponse.redirect(url);
}

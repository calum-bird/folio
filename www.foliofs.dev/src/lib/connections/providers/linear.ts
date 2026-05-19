import "server-only";

import type { ConnectorDefinition, OAuthAccount, OAuthTokenLike } from "@/lib/connections/registry";
import { requireEnv } from "@/lib/connections/registry";

type LinearTokenResponse = {
  access_token?: string;
  refresh_token?: string;
  expires_in?: number;
  token_type?: string;
  scope?: string;
  error?: string;
  error_description?: string;
};

type LinearViewerResponse = {
  data?: {
    viewer?: {
      id: string;
      name?: string;
      displayName?: string;
      email?: string;
    };
    organization?: {
      id: string;
      name: string;
      urlKey?: string;
    };
  };
  errors?: Array<{ message?: string }>;
};

export const linearConnector: ConnectorDefinition = {
  provider: "linear",
  displayName: "Linear",
  description: "Sync teams and issues into markdown files in your folio.",
  defaultScopes: "read",
  scopeEnvVar: "LINEAR_OAUTH_SCOPES",

  buildAuthorizeUrl({ state, redirectUri, scopes }) {
    const url = new URL("https://linear.app/oauth/authorize");
    url.searchParams.set("client_id", requireEnv("LINEAR_OAUTH_CLIENT_ID"));
    url.searchParams.set("redirect_uri", redirectUri);
    url.searchParams.set("response_type", "code");
    url.searchParams.set("scope", scopes);
    url.searchParams.set("state", state);
    return url;
  },

  async exchangeCode({ code, redirectUri }): Promise<OAuthTokenLike> {
    const body = new URLSearchParams({
      client_id: requireEnv("LINEAR_OAUTH_CLIENT_ID"),
      client_secret: requireEnv("LINEAR_OAUTH_CLIENT_SECRET"),
      code,
      grant_type: "authorization_code",
      redirect_uri: redirectUri,
    });
    const response = await fetch("https://api.linear.app/oauth/token", {
      method: "POST",
      headers: {
        accept: "application/json",
        "content-type": "application/x-www-form-urlencoded",
      },
      body,
    });

    if (!response.ok) {
      throw new Error(`Linear token exchange failed: ${response.status}`);
    }

    const payload = (await response.json()) as LinearTokenResponse;
    if (payload.error) {
      throw new Error(payload.error_description ?? payload.error);
    }
    if (!payload.access_token) {
      throw new Error("Linear token exchange did not return an access token");
    }

    return {
      accessToken: payload.access_token,
      refreshToken: payload.refresh_token,
      tokenType: payload.token_type,
      scope: payload.scope,
      expiresInSeconds: payload.expires_in,
    };
  },

  async fetchAccount(accessToken): Promise<OAuthAccount> {
    const response = await fetch("https://api.linear.app/graphql", {
      method: "POST",
      headers: {
        accept: "application/json",
        authorization: `Bearer ${accessToken}`,
        "content-type": "application/json",
        "user-agent": "FolioFS",
      },
      body: JSON.stringify({ query: VIEWER_QUERY }),
    });

    if (!response.ok) {
      throw new Error(`Linear workspace lookup failed: ${response.status}`);
    }

    const payload = (await response.json()) as LinearViewerResponse;
    const firstError = payload.errors?.find((entry) => entry.message)?.message;
    if (firstError) {
      throw new Error(firstError);
    }

    const organization = payload.data?.organization;
    if (!organization) {
      throw new Error("Linear workspace lookup did not return an organization");
    }

    return {
      id: organization.id,
      login: organization.urlKey ?? organization.name,
      displayName: organization.name,
    };
  },
};

const VIEWER_QUERY = `
  query FolioLinearWorkspace {
    viewer {
      id
      name
      displayName
      email
    }
    organization {
      id
      name
      urlKey
    }
  }
`;

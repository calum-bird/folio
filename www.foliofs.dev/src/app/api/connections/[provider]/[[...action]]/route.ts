import { auth } from "@clerk/nextjs/server";

import {
  completeOAuth,
  disconnectConnection,
  startOAuth,
} from "@/lib/connections/oauth";
import { getConnector } from "@/lib/connections/registry";

type RouteContext = {
  params: Promise<{
    provider: string;
    action?: string[];
  }>;
};

export async function GET(request: Request, context: RouteContext) {
  const { userId } = await auth();
  if (!userId) {
    return new Response("Unauthorized", { status: 401 });
  }

  const { provider, action = [] } = await context.params;
  const connector = getConnector(provider);
  if (!connector) {
    return new Response("Unknown provider", { status: 404 });
  }

  if (action.length === 1 && action[0] === "start") {
    return startOAuth(request, connector);
  }

  if (action.length === 1 && action[0] === "callback") {
    return completeOAuth(request, userId, connector);
  }

  return new Response("Not found", { status: 404 });
}

export async function POST(request: Request, context: RouteContext) {
  const { userId } = await auth();
  if (!userId) {
    return new Response("Unauthorized", { status: 401 });
  }

  const { provider, action = [] } = await context.params;
  const connector = getConnector(provider);
  if (!connector) {
    return new Response("Unknown provider", { status: 404 });
  }

  if (action.length === 2 && action[1] === "disconnect") {
    return disconnectConnection(request, userId, connector, action[0]);
  }

  return new Response("Method not allowed", { status: 405 });
}

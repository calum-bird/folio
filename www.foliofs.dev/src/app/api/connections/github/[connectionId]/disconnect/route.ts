import { auth } from "@clerk/nextjs/server";
import { NextResponse } from "next/server";

import { deleteConnectionSecret } from "@/lib/connections/secrets";
import { deleteConnection, getConnection } from "@/lib/connections/store";

type RouteContext = {
  params: Promise<{
    connectionId: string;
  }>;
};

export async function POST(_request: Request, context: RouteContext) {
  const { userId } = await auth();
  if (!userId) {
    return new Response("Unauthorized", { status: 401 });
  }

  const { connectionId } = await context.params;
  const connection = await getConnection(userId, "github", connectionId);
  if (!connection) {
    return NextResponse.redirect(new URL("/connections", _request.url), 303);
  }

  await deleteConnectionSecret(connection.secretArn);
  await deleteConnection(userId, "github", connectionId);

  return NextResponse.redirect(new URL("/connections?disconnected=github", _request.url), 303);
}

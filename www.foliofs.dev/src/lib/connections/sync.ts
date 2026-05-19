import "server-only";

import { SendMessageCommand } from "@aws-sdk/client-sqs";

import { sqsClient } from "@/lib/aws";
import { type ConnectionRecord } from "@/lib/connections/model";

export async function enqueueSync(connection: ConnectionRecord) {
  const job = {
    userId: connection.userId,
    connectionId: connection.connectionId,
    provider: connection.provider,
  };

  await sqsClient.send(
    new SendMessageCommand({
      QueueUrl: syncQueueUrl(),
      MessageBody: JSON.stringify(job),
    }),
  );
}

function syncQueueUrl() {
  const url = process.env.FOLIO_SYNC_QUEUE_URL;
  if (!url) {
    throw new Error("FOLIO_SYNC_QUEUE_URL is not configured");
  }

  return url;
}

import "server-only";

import {
  DeleteCommand,
  GetCommand,
  PutCommand,
  QueryCommand,
  UpdateCommand,
} from "@aws-sdk/lib-dynamodb";

import { documentClient } from "@/lib/aws";
import {
  type ConnectionProvider,
  type ConnectionRecord,
  type ConnectionStatus,
  connectionPk,
  connectionSk,
  connectionTableName,
} from "@/lib/connections/model";

export async function listUserConnections(userId: string) {
  const response = await documentClient.send(
    new QueryCommand({
      TableName: connectionTableName(),
      KeyConditionExpression: "pk = :pk AND begins_with(sk, :prefix)",
      ExpressionAttributeValues: {
        ":pk": connectionPk(userId),
        ":prefix": "CONNECTION#",
      },
    }),
  );

  return (response.Items ?? []) as ConnectionRecord[];
}

export async function getConnection(
  userId: string,
  provider: ConnectionProvider,
  connectionId: string,
) {
  const response = await documentClient.send(
    new GetCommand({
      TableName: connectionTableName(),
      Key: {
        pk: connectionPk(userId),
        sk: connectionSk(provider, connectionId),
      },
    }),
  );

  return response.Item as ConnectionRecord | undefined;
}

export async function putConnection(record: ConnectionRecord) {
  await documentClient.send(
    new PutCommand({
      TableName: connectionTableName(),
      Item: record,
    }),
  );
}

export async function deleteConnection(
  userId: string,
  provider: ConnectionProvider,
  connectionId: string,
) {
  await documentClient.send(
    new DeleteCommand({
      TableName: connectionTableName(),
      Key: {
        pk: connectionPk(userId),
        sk: connectionSk(provider, connectionId),
      },
    }),
  );
}

export async function updateConnectionStatus(
  userId: string,
  provider: ConnectionProvider,
  connectionId: string,
  status: ConnectionStatus,
) {
  const now = new Date().toISOString();
  await documentClient.send(
    new UpdateCommand({
      TableName: connectionTableName(),
      Key: {
        pk: connectionPk(userId),
        sk: connectionSk(provider, connectionId),
      },
      UpdateExpression: "SET #status = :status, updatedAt = :now REMOVE gsi1pk, gsi1sk",
      ExpressionAttributeNames: {
        "#status": "status",
      },
      ExpressionAttributeValues: {
        ":status": status,
        ":now": now,
      },
    }),
  );
}

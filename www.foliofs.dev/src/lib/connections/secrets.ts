import "server-only";

import {
  CreateSecretCommand,
  DeleteSecretCommand,
  PutSecretValueCommand,
  ResourceNotFoundException,
} from "@aws-sdk/client-secrets-manager";

import { secretsClient } from "@/lib/aws";
import { type ConnectionProvider, type ProviderTokenSecret, secretNameForConnection } from "@/lib/connections/model";

export async function putConnectionSecret(
  userId: string,
  provider: ConnectionProvider,
  connectionId: string,
  secret: ProviderTokenSecret,
) {
  const name = secretNameForConnection(userId, provider, connectionId);
  const payload = JSON.stringify(secret);

  try {
    const response = await secretsClient.send(
      new CreateSecretCommand({
        KmsKeyId: process.env.FOLIO_CONNECTION_SECRETS_KMS_KEY_ID,
        Name: name,
        SecretString: payload,
      }),
    );
    return response.ARN ?? name;
  } catch (error) {
    if (!(error instanceof Error) || error.name !== "ResourceExistsException") {
      throw error;
    }
  }

  const response = await secretsClient.send(
    new PutSecretValueCommand({
      SecretId: name,
      SecretString: payload,
    }),
  );

  return response.ARN ?? name;
}

export async function deleteConnectionSecret(secretArn: string) {
  try {
    await secretsClient.send(
      new DeleteSecretCommand({
        SecretId: secretArn,
        RecoveryWindowInDays: 7,
      }),
    );
  } catch (error) {
    if (error instanceof ResourceNotFoundException) {
      return;
    }

    throw error;
  }
}

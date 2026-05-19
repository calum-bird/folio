import "server-only";

import { EncryptCommand } from "@aws-sdk/client-kms";

import { kmsClient } from "@/lib/aws";
import { type ProviderTokenSecret } from "@/lib/connections/model";

export async function encryptConnectionToken(
  secret: ProviderTokenSecret,
  encryptionContext: Record<string, string>,
) {
  const response = await kmsClient.send(
    new EncryptCommand({
      KeyId: connectionSecretsKmsKeyId(),
      Plaintext: Buffer.from(JSON.stringify(secret), "utf8"),
      EncryptionContext: encryptionContext,
    }),
  );

  const ciphertext = response.CiphertextBlob;
  if (!ciphertext) {
    throw new Error("KMS encrypt returned no ciphertext");
  }

  return Buffer.from(ciphertext).toString("base64");
}

export function tokenEncryptionContext(pk: string, sk: string) {
  return { pk, sk };
}

function connectionSecretsKmsKeyId() {
  const keyId = process.env.FOLIO_CONNECTION_SECRETS_KMS_KEY_ID;
  if (!keyId) {
    throw new Error("FOLIO_CONNECTION_SECRETS_KMS_KEY_ID is not configured");
  }

  return keyId;
}

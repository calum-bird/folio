import "server-only";

import { DynamoDBClient } from "@aws-sdk/client-dynamodb";
import { SecretsManagerClient } from "@aws-sdk/client-secrets-manager";
import { fromIni } from "@aws-sdk/credential-providers";
import { DynamoDBDocumentClient } from "@aws-sdk/lib-dynamodb";

const clientConfig = {
  region: process.env.AWS_REGION ?? process.env.AWS_DEFAULT_REGION ?? "us-west-2",
  credentials: awsCredentials(),
};

const dynamodb = new DynamoDBClient(clientConfig);
const secrets = new SecretsManagerClient(clientConfig);

export const documentClient = DynamoDBDocumentClient.from(dynamodb, {
  marshallOptions: {
    removeUndefinedValues: true,
  },
});

export const secretsClient = secrets;

function awsCredentials() {
  const profile = process.env.AWS_PROFILE;
  if (!profile) {
    return undefined;
  }

  return fromIni({ profile });
}

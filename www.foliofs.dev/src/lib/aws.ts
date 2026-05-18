import "server-only";

import { DynamoDBClient } from "@aws-sdk/client-dynamodb";
import { SecretsManagerClient } from "@aws-sdk/client-secrets-manager";
import { DynamoDBDocumentClient } from "@aws-sdk/lib-dynamodb";

const dynamodb = new DynamoDBClient({});
const secrets = new SecretsManagerClient({});

export const documentClient = DynamoDBDocumentClient.from(dynamodb, {
  marshallOptions: {
    removeUndefinedValues: true,
  },
});

export const secretsClient = secrets;

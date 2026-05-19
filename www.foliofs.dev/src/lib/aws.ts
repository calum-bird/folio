import "server-only";

import { DynamoDBClient } from "@aws-sdk/client-dynamodb";
import { KMSClient } from "@aws-sdk/client-kms";
import { SQSClient } from "@aws-sdk/client-sqs";
import { fromIni } from "@aws-sdk/credential-providers";
import { DynamoDBDocumentClient } from "@aws-sdk/lib-dynamodb";

const clientConfig = {
  region: process.env.AWS_REGION ?? process.env.AWS_DEFAULT_REGION ?? "us-west-2",
  credentials: awsCredentials(),
};

const dynamodb = new DynamoDBClient(clientConfig);
const kms = new KMSClient(clientConfig);
const sqs = new SQSClient(clientConfig);

export const documentClient = DynamoDBDocumentClient.from(dynamodb, {
  marshallOptions: {
    removeUndefinedValues: true,
  },
});

export const kmsClient = kms;
export const sqsClient = sqs;

function awsCredentials() {
  const profile = process.env.AWS_PROFILE;
  if (!profile) {
    return undefined;
  }

  return fromIni({ profile });
}

#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TERRAFORM_DIR="$ROOT_DIR/infra/aws"

AWS_PROFILE="${AWS_PROFILE:-calum}"
AWS_REGION="${AWS_REGION:-us-west-2}"
DOMAIN_NAME="${DOMAIN_NAME:-api.foliofs.dev}"
WAIT_FOR_CERTIFICATE_VALIDATION="${WAIT_FOR_CERTIFICATE_VALIDATION:-true}"
PLATFORM="${PLATFORM:-linux/arm64}"

export AWS_PROFILE

terraform_output() {
  terraform -chdir="$TERRAFORM_DIR" output -raw "$1"
}

ecr_digest() {
  local repository_name="$1"
  AWS_PROFILE="$AWS_PROFILE" aws ecr describe-images \
    --region "$AWS_REGION" \
    --repository-name "$repository_name" \
    --image-ids imageTag=latest \
    --query 'imageDetails[0].imageDigest' \
    --output text
}

build_and_push() {
  local dockerfile="$1"
  local repository="$2"

  docker buildx build \
    --platform "$PLATFORM" \
    --provenance=false \
    --sbom=false \
    -f "$dockerfile" \
    -t "${repository}:latest" \
    --push \
    "$ROOT_DIR"
}

echo "Using AWS_PROFILE=$AWS_PROFILE AWS_REGION=$AWS_REGION"

APP_REPO="$(terraform_output ecr_repository_url)"
DISPATCHER_REPO="$(terraform_output sync_dispatcher_ecr_repository_url)"
WORKER_REPO="$(terraform_output sync_worker_ecr_repository_url)"
ECR_REGISTRY="${APP_REPO%/*}"

AWS_PROFILE="$AWS_PROFILE" aws ecr get-login-password --region "$AWS_REGION" \
  | docker login --username AWS --password-stdin "$ECR_REGISTRY"

echo "Building and pushing WebDAV server image..."
build_and_push "$ROOT_DIR/Dockerfile" "$APP_REPO"

echo "Building and pushing sync dispatcher image..."
build_and_push "$ROOT_DIR/Dockerfile.sync-dispatcher" "$DISPATCHER_REPO"

echo "Building and pushing sync worker image..."
build_and_push "$ROOT_DIR/Dockerfile.sync-worker" "$WORKER_REPO"

DISPATCHER_DIGEST="$(ecr_digest foliofs-sync-dispatcher)"
WORKER_DIGEST="$(ecr_digest foliofs-sync-worker)"

echo "Applying Terraform with immutable Lambda image digests..."
AWS_PROFILE="$AWS_PROFILE" terraform -chdir="$TERRAFORM_DIR" apply -auto-approve \
  -var "aws_region=$AWS_REGION" \
  -var "domain_name=$DOMAIN_NAME" \
  -var "wait_for_certificate_validation=$WAIT_FOR_CERTIFICATE_VALIDATION" \
  -var "container_image=${APP_REPO}:latest" \
  -var "sync_dispatcher_image=${DISPATCHER_REPO}@${DISPATCHER_DIGEST}" \
  -var "sync_worker_image=${WORKER_REPO}@${WORKER_DIGEST}"

ECS_CLUSTER="$(terraform_output ecs_cluster_name)"
ECS_SERVICE="$(terraform_output ecs_service_name)"

echo "Forcing ECS WebDAV service deployment..."
AWS_PROFILE="$AWS_PROFILE" aws ecs update-service \
  --region "$AWS_REGION" \
  --cluster "$ECS_CLUSTER" \
  --service "$ECS_SERVICE" \
  --force-new-deployment \
  >/dev/null

echo "Backend deploy complete."

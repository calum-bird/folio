output "alb_dns_name" {
  description = "DNS name of the public application load balancer."
  value       = aws_lb.app.dns_name
}

output "domain_name" {
  description = "DNS name configured for the ALB, if enabled."
  value       = var.domain_name
}

output "domain_name_servers" {
  description = "Name servers to delegate the configured DNS zone to."
  value       = try(aws_route53_zone.app[0].name_servers, [])
}

output "certificate_arn" {
  description = "ACM certificate ARN for the configured DNS name, if enabled."
  value       = try(aws_acm_certificate.app[0].arn, "")
}

output "ecr_repository_url" {
  description = "ECR repository URL for the FolioFS server image."
  value       = aws_ecr_repository.app.repository_url
}

output "sync_dispatcher_ecr_repository_url" {
  description = "ECR repository URL for the sync dispatcher Lambda image."
  value       = aws_ecr_repository.sync_dispatcher.repository_url
}

output "sync_worker_ecr_repository_url" {
  description = "ECR repository URL for the sync worker Lambda image."
  value       = aws_ecr_repository.sync_worker.repository_url
}

output "s3_bucket_name" {
  description = "S3 bucket backing the S3 Files file system."
  value       = aws_s3_bucket.data.bucket
}

output "s3files_file_system_id" {
  description = "S3 Files file system mounted into the ECS task."
  value       = aws_s3files_file_system.data.id
}

output "ecs_cluster_name" {
  description = "ECS cluster name."
  value       = aws_ecs_cluster.main.name
}

output "ecs_service_name" {
  description = "ECS service name."
  value       = aws_ecs_service.app.name
}

output "connections_table_name" {
  description = "DynamoDB table used for FolioFS provider connections."
  value       = aws_dynamodb_table.connections.name
}

output "connection_secrets_kms_key_arn" {
  description = "KMS key used for FolioFS provider connection secrets."
  value       = aws_kms_key.connection_secrets.arn
}

output "sync_queue_url" {
  description = "SQS queue URL for FolioFS sync jobs."
  value       = aws_sqs_queue.sync.url
}

output "sync_dispatcher_lambda_name" {
  description = "Lambda function name for dispatching due sync jobs."
  value       = aws_lambda_function.sync_dispatcher.function_name
}

output "sync_worker_lambda_name" {
  description = "Lambda function name for processing sync jobs."
  value       = aws_lambda_function.sync_worker.function_name
}

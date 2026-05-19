data "aws_availability_zones" "available" {
  state = "available"
}

data "aws_caller_identity" "current" {}

locals {
  container_name            = "foliofs-dav-server"
  container_port            = 4918
  name                      = lower(replace(var.name, "_", "-"))
  domain_enabled            = var.domain_name != ""
  ecr_image                 = "${aws_ecr_repository.app.repository_url}:latest"
  sync_dispatcher_ecr_image = "${aws_ecr_repository.sync_dispatcher.repository_url}:latest"
  sync_worker_ecr_image     = "${aws_ecr_repository.sync_worker.repository_url}:latest"
  container_image           = var.container_image != "" ? var.container_image : local.ecr_image
  sync_dispatcher_image     = var.sync_dispatcher_image != "" ? var.sync_dispatcher_image : local.sync_dispatcher_ecr_image
  sync_worker_image         = var.sync_worker_image != "" ? var.sync_worker_image : local.sync_worker_ecr_image
  bucket_name               = substr("${local.name}-${data.aws_caller_identity.current.account_id}-${var.aws_region}", 0, 63)
  connections_table_name    = "${local.name}-connections"
  managed_certificate_arn   = try(aws_acm_certificate_validation.app[0].certificate_arn, "")
  effective_certificate_arn = var.certificate_arn != "" ? var.certificate_arn : local.managed_certificate_arn
  https_enabled             = local.effective_certificate_arn != ""
}

resource "aws_vpc" "main" {
  cidr_block           = var.vpc_cidr
  enable_dns_hostnames = true
  enable_dns_support   = true

  tags = {
    Name = local.name
  }
}

resource "aws_internet_gateway" "main" {
  vpc_id = aws_vpc.main.id

  tags = {
    Name = local.name
  }
}

resource "aws_subnet" "public" {
  count = length(var.public_subnet_cidrs)

  vpc_id                  = aws_vpc.main.id
  cidr_block              = var.public_subnet_cidrs[count.index]
  availability_zone       = data.aws_availability_zones.available.names[count.index]
  map_public_ip_on_launch = true

  tags = {
    Name = "${local.name}-public-${count.index + 1}"
  }
}

resource "aws_subnet" "private" {
  count = length(var.private_subnet_cidrs)

  vpc_id                  = aws_vpc.main.id
  cidr_block              = var.private_subnet_cidrs[count.index]
  availability_zone       = data.aws_availability_zones.available.names[count.index]
  map_public_ip_on_launch = false

  tags = {
    Name = "${local.name}-private-${count.index + 1}"
  }
}

resource "aws_eip" "nat" {
  domain = "vpc"

  tags = {
    Name = "${local.name}-nat"
  }
}

resource "aws_nat_gateway" "main" {
  allocation_id = aws_eip.nat.id
  subnet_id     = aws_subnet.public[0].id

  tags = {
    Name = "${local.name}-nat"
  }

  depends_on = [
    aws_internet_gateway.main
  ]
}

resource "aws_route_table" "public" {
  vpc_id = aws_vpc.main.id

  route {
    cidr_block = "0.0.0.0/0"
    gateway_id = aws_internet_gateway.main.id
  }

  tags = {
    Name = "${local.name}-public"
  }
}

resource "aws_route_table_association" "public" {
  count = length(aws_subnet.public)

  subnet_id      = aws_subnet.public[count.index].id
  route_table_id = aws_route_table.public.id
}

resource "aws_route_table" "private" {
  vpc_id = aws_vpc.main.id

  route {
    cidr_block     = "0.0.0.0/0"
    nat_gateway_id = aws_nat_gateway.main.id
  }

  tags = {
    Name = "${local.name}-private"
  }
}

resource "aws_route_table_association" "private" {
  count = length(aws_subnet.private)

  subnet_id      = aws_subnet.private[count.index].id
  route_table_id = aws_route_table.private.id
}

resource "aws_security_group" "alb" {
  name        = "${local.name}-alb"
  description = "Public access to the FolioFS load balancer."
  vpc_id      = aws_vpc.main.id

  ingress {
    description = "HTTP"
    from_port   = 80
    to_port     = 80
    protocol    = "tcp"
    cidr_blocks = var.allowed_cidr_blocks
  }

  ingress {
    description = "HTTPS"
    from_port   = 443
    to_port     = 443
    protocol    = "tcp"
    cidr_blocks = var.allowed_cidr_blocks
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = {
    Name = "${local.name}-alb"
  }
}

resource "aws_security_group" "managed_instances" {
  name        = "${local.name}-managed-instances"
  description = "ECS Managed Instances hosting FolioFS tasks."
  vpc_id      = aws_vpc.main.id

  ingress {
    description     = "WebDAV from ALB"
    from_port       = local.container_port
    to_port         = local.container_port
    protocol        = "tcp"
    security_groups = [aws_security_group.alb.id]
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = {
    Name = "${local.name}-managed-instances"
  }
}

resource "aws_security_group" "s3files" {
  name        = "${local.name}-s3files"
  description = "S3 Files mount target access from ECS Managed Instances."
  vpc_id      = aws_vpc.main.id

  ingress {
    description     = "NFS from ECS Managed Instances"
    from_port       = 2049
    to_port         = 2049
    protocol        = "tcp"
    security_groups = [aws_security_group.managed_instances.id]
  }

  ingress {
    description     = "NFS from Lambda sync functions"
    from_port       = 2049
    to_port         = 2049
    protocol        = "tcp"
    security_groups = [aws_security_group.lambda.id]
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = {
    Name = "${local.name}-s3files"
  }
}

resource "aws_security_group" "lambda" {
  name        = "${local.name}-lambda"
  description = "Lambda sync functions."
  vpc_id      = aws_vpc.main.id

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = {
    Name = "${local.name}-lambda"
  }
}

resource "aws_lb" "app" {
  name               = local.name
  load_balancer_type = "application"
  security_groups    = [aws_security_group.alb.id]
  subnets            = aws_subnet.public[*].id

  tags = {
    Name = local.name
  }
}

resource "aws_lb_target_group" "app" {
  name        = local.name
  port        = local.container_port
  protocol    = "HTTP"
  target_type = "instance"
  vpc_id      = aws_vpc.main.id

  health_check {
    enabled             = true
    healthy_threshold   = 2
    interval            = 30
    matcher             = "200"
    path                = "/healthz"
    timeout             = 5
    unhealthy_threshold = 2
  }
}

resource "aws_lb_listener" "http" {
  load_balancer_arn = aws_lb.app.arn
  port              = 80
  protocol          = "HTTP"

  dynamic "default_action" {
    for_each = local.https_enabled ? [] : [1]

    content {
      type             = "forward"
      target_group_arn = aws_lb_target_group.app.arn
    }
  }

  dynamic "default_action" {
    for_each = local.https_enabled ? [1] : []

    content {
      type = "redirect"

      redirect {
        port        = "443"
        protocol    = "HTTPS"
        status_code = "HTTP_301"
      }
    }
  }
}

resource "aws_lb_listener" "https" {
  count = local.https_enabled ? 1 : 0

  load_balancer_arn = aws_lb.app.arn
  port              = 443
  protocol          = "HTTPS"
  certificate_arn   = local.effective_certificate_arn

  default_action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.app.arn
  }
}

resource "aws_route53_zone" "app" {
  count = local.domain_enabled ? 1 : 0

  name = var.domain_name
}

resource "aws_acm_certificate" "app" {
  count = local.domain_enabled ? 1 : 0

  domain_name       = var.domain_name
  validation_method = "DNS"

  lifecycle {
    create_before_destroy = true
  }
}

resource "aws_route53_record" "app" {
  count = local.domain_enabled ? 1 : 0

  zone_id = aws_route53_zone.app[0].zone_id
  name    = var.domain_name
  type    = "A"

  alias {
    evaluate_target_health = true
    name                   = aws_lb.app.dns_name
    zone_id                = aws_lb.app.zone_id
  }
}

resource "aws_route53_record" "certificate_validation" {
  for_each = local.domain_enabled ? {
    for option in aws_acm_certificate.app[0].domain_validation_options : option.domain_name => {
      name   = option.resource_record_name
      record = option.resource_record_value
      type   = option.resource_record_type
    }
  } : {}

  allow_overwrite = true
  name            = each.value.name
  records         = [each.value.record]
  ttl             = 60
  type            = each.value.type
  zone_id         = aws_route53_zone.app[0].zone_id
}

resource "aws_acm_certificate_validation" "app" {
  count = local.domain_enabled && var.wait_for_certificate_validation ? 1 : 0

  certificate_arn         = aws_acm_certificate.app[0].arn
  validation_record_fqdns = [for record in aws_route53_record.certificate_validation : record.fqdn]
}

resource "aws_ecr_repository" "app" {
  name                 = local.name
  image_tag_mutability = "MUTABLE"

  image_scanning_configuration {
    scan_on_push = true
  }
}

resource "aws_ecr_repository" "sync_dispatcher" {
  name                 = "${local.name}-sync-dispatcher"
  image_tag_mutability = "MUTABLE"

  image_scanning_configuration {
    scan_on_push = true
  }
}

resource "aws_ecr_repository" "sync_worker" {
  name                 = "${local.name}-sync-worker"
  image_tag_mutability = "MUTABLE"

  image_scanning_configuration {
    scan_on_push = true
  }
}

resource "aws_cloudwatch_log_group" "app" {
  name              = "/ecs/${local.name}"
  retention_in_days = 14
}

resource "aws_dynamodb_table" "connections" {
  name         = local.connections_table_name
  billing_mode = "PAY_PER_REQUEST"
  hash_key     = "pk"
  range_key    = "sk"

  attribute {
    name = "pk"
    type = "S"
  }

  attribute {
    name = "sk"
    type = "S"
  }

  attribute {
    name = "gsi1pk"
    type = "S"
  }

  attribute {
    name = "gsi1sk"
    type = "S"
  }

  global_secondary_index {
    name            = "gsi1"
    hash_key        = "gsi1pk"
    range_key       = "gsi1sk"
    projection_type = "ALL"
  }

  point_in_time_recovery {
    enabled = true
  }

  server_side_encryption {
    enabled = true
  }

  tags = {
    Name = local.connections_table_name
  }
}

resource "aws_sqs_queue" "sync_dlq" {
  name                      = "${local.name}-sync-dlq"
  message_retention_seconds = 1209600
}

resource "aws_sqs_queue" "sync" {
  name                       = "${local.name}-sync"
  visibility_timeout_seconds = 960
  message_retention_seconds  = 1209600

  redrive_policy = jsonencode({
    deadLetterTargetArn = aws_sqs_queue.sync_dlq.arn
    maxReceiveCount     = 3
  })
}

resource "aws_kms_key" "connection_secrets" {
  description             = "Encrypt FolioFS provider OAuth connection secrets."
  deletion_window_in_days = 7
  enable_key_rotation     = true
}

resource "aws_kms_alias" "connection_secrets" {
  name          = "alias/${local.name}-connection-secrets"
  target_key_id = aws_kms_key.connection_secrets.key_id
}

resource "aws_s3_bucket" "data" {
  bucket = local.bucket_name
}

resource "aws_s3_bucket_public_access_block" "data" {
  bucket = aws_s3_bucket.data.id

  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

resource "aws_s3_bucket_versioning" "data" {
  bucket = aws_s3_bucket.data.id

  versioning_configuration {
    status = "Enabled"
  }
}

resource "aws_s3_bucket_server_side_encryption_configuration" "data" {
  bucket = aws_s3_bucket.data.id

  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm = "AES256"
    }
  }
}

resource "aws_iam_role" "s3files" {
  name = "${local.name}-s3files"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Sid    = "AllowS3FilesAssumeRole"
      Effect = "Allow"
      Principal = {
        Service = "elasticfilesystem.amazonaws.com"
      }
      Action = "sts:AssumeRole"
      Condition = {
        StringEquals = {
          "aws:SourceAccount" = data.aws_caller_identity.current.account_id
        }
        ArnLike = {
          "aws:SourceArn" = "arn:aws:s3files:${var.aws_region}:${data.aws_caller_identity.current.account_id}:file-system/*"
        }
      }
    }]
  })
}

resource "aws_iam_role_policy" "s3files" {
  name = "${local.name}-s3files"
  role = aws_iam_role.s3files.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Sid    = "S3BucketPermissions"
        Effect = "Allow"
        Action = [
          "s3:ListBucket",
          "s3:ListBucketVersions"
        ]
        Resource = aws_s3_bucket.data.arn
        Condition = {
          StringEquals = {
            "aws:ResourceAccount" = data.aws_caller_identity.current.account_id
          }
        }
      },
      {
        Sid    = "S3ObjectPermissions"
        Effect = "Allow"
        Action = [
          "s3:AbortMultipartUpload",
          "s3:DeleteObject*",
          "s3:GetObject*",
          "s3:List*",
          "s3:PutObject*"
        ]
        Resource = "${aws_s3_bucket.data.arn}/*"
        Condition = {
          StringEquals = {
            "aws:ResourceAccount" = data.aws_caller_identity.current.account_id
          }
        }
      },
      {
        Sid    = "EventBridgeManage"
        Effect = "Allow"
        Action = [
          "events:DeleteRule",
          "events:DisableRule",
          "events:EnableRule",
          "events:PutRule",
          "events:PutTargets",
          "events:RemoveTargets"
        ]
        Resource = "arn:aws:events:*:*:rule/DO-NOT-DELETE-S3-Files*"
        Condition = {
          StringEquals = {
            "events:ManagedBy" = "elasticfilesystem.amazonaws.com"
          }
        }
      },
      {
        Sid    = "EventBridgeRead"
        Effect = "Allow"
        Action = [
          "events:DescribeRule",
          "events:ListRuleNamesByTarget",
          "events:ListRules",
          "events:ListTargetsByRule"
        ]
        Resource = "arn:aws:events:*:*:rule/*"
      }
    ]
  })
}

resource "aws_s3files_file_system" "data" {
  bucket   = aws_s3_bucket.data.arn
  role_arn = aws_iam_role.s3files.arn

  depends_on = [
    aws_iam_role_policy.s3files,
    aws_s3_bucket_server_side_encryption_configuration.data,
    aws_s3_bucket_versioning.data
  ]

  tags = {
    Name = local.name
  }
}

resource "aws_s3files_access_point" "lambda" {
  file_system_id = aws_s3files_file_system.data.id

  posix_user {
    gid = 0
    uid = 0
  }

  root_directory {
    path = "/"
  }

  tags = {
    Name = "${local.name}-lambda"
  }
}

resource "aws_s3files_mount_target" "data" {
  count = length(aws_subnet.public)

  file_system_id = aws_s3files_file_system.data.id
  security_groups = [
    aws_security_group.s3files.id
  ]
  subnet_id = aws_subnet.public[count.index].id
}

resource "aws_iam_role" "ecs_infrastructure" {
  name = "${local.name}-ecs-infrastructure"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect = "Allow"
      Principal = {
        Service = "ecs.amazonaws.com"
      }
      Action = "sts:AssumeRole"
    }]
  })
}

resource "aws_iam_role_policy_attachment" "ecs_infrastructure" {
  role       = aws_iam_role.ecs_infrastructure.name
  policy_arn = "arn:aws:iam::aws:policy/AmazonECSInfrastructureRolePolicyForManagedInstances"
}

resource "aws_iam_role" "ecs_instance" {
  name = "ecsInstanceRole-${local.name}"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect = "Allow"
      Principal = {
        Service = "ec2.amazonaws.com"
      }
      Action = "sts:AssumeRole"
    }]
  })
}

resource "aws_iam_role_policy_attachment" "ecs_instance" {
  role       = aws_iam_role.ecs_instance.name
  policy_arn = "arn:aws:iam::aws:policy/AmazonECSInstanceRolePolicyForManagedInstances"
}

resource "aws_iam_instance_profile" "ecs_instance" {
  name = aws_iam_role.ecs_instance.name
  role = aws_iam_role.ecs_instance.name
}

resource "aws_iam_role" "task_execution" {
  name = "${local.name}-task-execution"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect = "Allow"
      Principal = {
        Service = "ecs-tasks.amazonaws.com"
      }
      Action = "sts:AssumeRole"
    }]
  })
}

resource "aws_iam_role_policy_attachment" "task_execution" {
  role       = aws_iam_role.task_execution.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AmazonECSTaskExecutionRolePolicy"
}

resource "aws_iam_role" "task" {
  name = "${local.name}-task"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect = "Allow"
      Principal = {
        Service = "ecs-tasks.amazonaws.com"
      }
      Action = "sts:AssumeRole"
    }]
  })
}

resource "aws_iam_role_policy_attachment" "task_s3files" {
  role       = aws_iam_role.task.name
  policy_arn = "arn:aws:iam::aws:policy/AmazonS3FilesClientFullAccess"
}

resource "aws_iam_role_policy" "task_bucket" {
  name = "${local.name}-task-bucket"
  role = aws_iam_role.task.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Sid    = "S3ObjectAccess"
        Effect = "Allow"
        Action = [
          "s3:AbortMultipartUpload",
          "s3:DeleteObject",
          "s3:GetObject",
          "s3:GetObjectVersion",
          "s3:PutObject"
        ]
        Resource = "${aws_s3_bucket.data.arn}/*"
      },
      {
        Sid    = "S3BucketListAccess"
        Effect = "Allow"
        Action = [
          "s3:GetBucketLocation",
          "s3:ListBucket"
        ]
        Resource = aws_s3_bucket.data.arn
      }
    ]
  })
}

resource "aws_iam_role" "sync_lambda" {
  name = "${local.name}-sync-lambda"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect = "Allow"
      Principal = {
        Service = "lambda.amazonaws.com"
      }
      Action = "sts:AssumeRole"
    }]
  })
}

resource "aws_iam_role_policy_attachment" "sync_lambda_basic" {
  role       = aws_iam_role.sync_lambda.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole"
}

resource "aws_iam_role_policy_attachment" "sync_lambda_vpc" {
  role       = aws_iam_role.sync_lambda.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AWSLambdaVPCAccessExecutionRole"
}

resource "aws_iam_role_policy" "sync_lambda" {
  name = "${local.name}-sync-lambda"
  role = aws_iam_role.sync_lambda.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Sid    = "ConnectionTableAccess"
        Effect = "Allow"
        Action = [
          "dynamodb:GetItem",
          "dynamodb:Query",
          "dynamodb:UpdateItem"
        ]
        Resource = [
          aws_dynamodb_table.connections.arn,
          "${aws_dynamodb_table.connections.arn}/index/*"
        ]
      },
      {
        Sid    = "SyncQueueAccess"
        Effect = "Allow"
        Action = [
          "sqs:DeleteMessage",
          "sqs:GetQueueAttributes",
          "sqs:ReceiveMessage",
          "sqs:SendMessage"
        ]
        Resource = aws_sqs_queue.sync.arn
      },
      {
        Sid      = "SyncDeadLetterQueueAccess"
        Effect   = "Allow"
        Action   = ["sqs:SendMessage"]
        Resource = aws_sqs_queue.sync_dlq.arn
      },
      {
        Sid    = "ConnectionTokenKmsDecrypt"
        Effect = "Allow"
        Action = ["kms:Decrypt"]
        Resource = aws_kms_key.connection_secrets.arn
      },
      {
        Sid    = "S3FilesMountAccess"
        Effect = "Allow"
        Action = [
          "s3files:ClientMount",
          "s3files:ClientWrite"
        ]
        Resource = "*"
      },
      {
        Sid    = "S3DirectRead"
        Effect = "Allow"
        Action = [
          "s3:GetObject",
          "s3:GetObjectVersion"
        ]
        Resource = "${aws_s3_bucket.data.arn}/*"
      }
    ]
  })
}

resource "aws_iam_role" "sync_scheduler" {
  name = "${local.name}-sync-scheduler"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect = "Allow"
      Principal = {
        Service = "scheduler.amazonaws.com"
      }
      Action = "sts:AssumeRole"
    }]
  })
}

resource "aws_iam_role_policy" "sync_scheduler" {
  name = "${local.name}-sync-scheduler"
  role = aws_iam_role.sync_scheduler.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect   = "Allow"
      Action   = "lambda:InvokeFunction"
      Resource = aws_lambda_function.sync_dispatcher.arn
    }]
  })
}

resource "aws_cloudwatch_log_group" "sync_dispatcher" {
  name              = "/aws/lambda/${local.name}-sync-dispatcher"
  retention_in_days = 14
}

resource "aws_cloudwatch_log_group" "sync_worker" {
  name              = "/aws/lambda/${local.name}-sync-worker"
  retention_in_days = 14
}

resource "aws_lambda_function" "sync_dispatcher" {
  function_name = "${local.name}-sync-dispatcher"
  package_type  = "Image"
  image_uri     = local.sync_dispatcher_image
  role          = aws_iam_role.sync_lambda.arn
  architectures = ["arm64"]
  memory_size   = 512
  timeout       = 60

  environment {
    variables = {
      FOLIO_CONNECTIONS_TABLE        = aws_dynamodb_table.connections.name
      FOLIO_SYNC_QUEUE_URL           = aws_sqs_queue.sync.url
      FOLIO_SYNC_INTERVAL_SECONDS    = tostring(var.sync_interval_seconds)
      FOLIO_SYNC_DISPATCH_BATCH_SIZE = "100"
    }
  }

  depends_on = [
    aws_cloudwatch_log_group.sync_dispatcher,
    aws_iam_role_policy.sync_lambda,
    aws_iam_role_policy_attachment.sync_lambda_basic
  ]
}

resource "aws_lambda_function" "sync_worker" {
  function_name = "${local.name}-sync-worker"
  package_type  = "Image"
  image_uri     = local.sync_worker_image
  role          = aws_iam_role.sync_lambda.arn
  architectures = ["arm64"]
  memory_size   = 1024
  timeout       = 900

  environment {
    variables = {
      FOLIO_CONNECTIONS_TABLE        = aws_dynamodb_table.connections.name
      FOLIO_DATA_ROOT                = "/mnt/folio"
      FOLIO_SYNC_INTERVAL_SECONDS    = tostring(var.sync_interval_seconds)
      FOLIO_TOKEN_CACHE_TTL_SECONDS  = "300"
    }
  }

  file_system_config {
    arn              = aws_s3files_access_point.lambda.arn
    local_mount_path = "/mnt/folio"
  }

  vpc_config {
    security_group_ids = [aws_security_group.lambda.id]
    subnet_ids         = aws_subnet.private[*].id
  }

  depends_on = [
    aws_cloudwatch_log_group.sync_worker,
    aws_iam_role_policy.sync_lambda,
    aws_iam_role_policy_attachment.sync_lambda_basic,
    aws_iam_role_policy_attachment.sync_lambda_vpc,
    aws_s3files_mount_target.data
  ]
}

resource "aws_lambda_event_source_mapping" "sync_worker" {
  event_source_arn = aws_sqs_queue.sync.arn
  function_name    = aws_lambda_function.sync_worker.arn
  batch_size       = 5
  enabled          = true
}

resource "aws_scheduler_schedule" "sync_dispatcher" {
  name                = "${local.name}-sync-dispatcher"
  schedule_expression = var.sync_dispatcher_rate
  state               = "DISABLED"

  flexible_time_window {
    mode = "OFF"
  }

  target {
    arn      = aws_lambda_function.sync_dispatcher.arn
    role_arn = aws_iam_role.sync_scheduler.arn

    retry_policy {
      maximum_event_age_in_seconds = 86400
      maximum_retry_attempts       = 0
    }
  }
}

resource "aws_ecs_cluster" "main" {
  name = local.name
}

resource "aws_ecs_capacity_provider" "managed_instances" {
  name    = "${local.name}-managed-instances"
  cluster = aws_ecs_cluster.main.name

  managed_instances_provider {
    infrastructure_role_arn = aws_iam_role.ecs_infrastructure.arn
    propagate_tags          = "CAPACITY_PROVIDER"

    instance_launch_template {
      ec2_instance_profile_arn = aws_iam_instance_profile.ecs_instance.arn
      monitoring               = "BASIC"

      network_configuration {
        security_groups = [aws_security_group.managed_instances.id]
        subnets         = aws_subnet.public[*].id
      }

      storage_configuration {
        storage_size_gib = 30
      }

      instance_requirements {
        allowed_instance_types = var.allowed_instance_types
        burstable_performance  = "included"

        memory_mib {
          max = var.instance_memory_mib_max
          min = var.instance_memory_mib_min
        }

        vcpu_count {
          max = var.instance_vcpu_max
          min = var.instance_vcpu_min
        }
      }
    }
  }

  depends_on = [
    aws_iam_role_policy_attachment.ecs_infrastructure,
    aws_iam_role_policy_attachment.ecs_instance
  ]
}

resource "aws_ecs_cluster_capacity_providers" "main" {
  cluster_name = aws_ecs_cluster.main.name

  capacity_providers = [
    aws_ecs_capacity_provider.managed_instances.name
  ]

  default_capacity_provider_strategy {
    capacity_provider = aws_ecs_capacity_provider.managed_instances.name
    weight            = 1
  }
}

resource "aws_ecs_task_definition" "app" {
  family                   = local.name
  cpu                      = tostring(var.container_cpu)
  memory                   = tostring(var.container_memory)
  network_mode             = "host"
  requires_compatibilities = ["MANAGED_INSTANCES"]
  execution_role_arn       = aws_iam_role.task_execution.arn
  task_role_arn            = aws_iam_role.task.arn

  runtime_platform {
    cpu_architecture        = "ARM64"
    operating_system_family = "LINUX"
  }

  volume {
    name = "data"

    s3files_volume_configuration {
      file_system_arn = aws_s3files_file_system.data.arn
      root_directory  = "/"
    }
  }

  container_definitions = jsonencode([
    {
      name      = local.container_name
      image     = local.container_image
      essential = true
      command = [
        "--bind",
        "0.0.0.0:${local.container_port}",
        "--root",
        "/data"
      ]
      environment = [
        {
          name  = "RUST_LOG"
          value = "info,foliofs_dav_server=debug"
        }
      ]
      logConfiguration = {
        logDriver = "awslogs"
        options = {
          awslogs-group         = aws_cloudwatch_log_group.app.name
          awslogs-region        = var.aws_region
          awslogs-stream-prefix = "dav-server"
        }
      }
      mountPoints = [
        {
          containerPath = "/data"
          readOnly      = false
          sourceVolume  = "data"
        }
      ]
      portMappings = [
        {
          containerPort = local.container_port
          hostPort      = local.container_port
          protocol      = "tcp"
        }
      ]
    }
  ])

  depends_on = [
    aws_s3files_mount_target.data
  ]
}

resource "aws_ecs_service" "app" {
  name            = local.name
  cluster         = aws_ecs_cluster.main.id
  task_definition = aws_ecs_task_definition.app.arn
  desired_count   = var.desired_count

  capacity_provider_strategy {
    capacity_provider = aws_ecs_capacity_provider.managed_instances.name
    weight            = 1
  }

  load_balancer {
    container_name   = local.container_name
    container_port   = local.container_port
    target_group_arn = aws_lb_target_group.app.arn
  }

  placement_constraints {
    type = "distinctInstance"
  }

  depends_on = [
    aws_ecs_cluster_capacity_providers.main,
    aws_lb_listener.http
  ]
}

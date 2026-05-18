variable "name" {
  description = "Short name used to prefix AWS resources."
  type        = string
  default     = "foliofs"
}

variable "aws_region" {
  description = "AWS region to deploy into."
  type        = string
  default     = "us-east-1"
}

variable "vpc_cidr" {
  description = "CIDR block for the VPC."
  type        = string
  default     = "10.42.0.0/16"
}

variable "public_subnet_cidrs" {
  description = "Public subnet CIDRs for the ALB and ECS Managed Instances."
  type        = list(string)
  default     = ["10.42.0.0/24", "10.42.1.0/24"]
}

variable "allowed_cidr_blocks" {
  description = "CIDR blocks allowed to reach the public load balancer."
  type        = list(string)
  default     = ["0.0.0.0/0"]
}

variable "certificate_arn" {
  description = "Optional existing ACM certificate ARN for an HTTPS listener."
  type        = string
  default     = ""
}

variable "domain_name" {
  description = "Optional DNS name to create in Route 53 and point at the ALB."
  type        = string
  default     = ""
}

variable "wait_for_certificate_validation" {
  description = "Wait for the managed ACM certificate to validate and enable HTTPS. Requires parent DNS delegation to already be in place."
  type        = bool
  default     = false
}

variable "container_image" {
  description = "Container image to run. Defaults to the Terraform-created ECR repo with the :latest tag."
  type        = string
  default     = ""
}

variable "desired_count" {
  description = "Number of WebDAV tasks to run."
  type        = number
  default     = 1
}

variable "allowed_instance_types" {
  description = "ARM instance types ECS Managed Instances may launch."
  type        = list(string)
  default     = ["t4g.medium"]
}

variable "instance_vcpu_min" {
  description = "Minimum vCPU count for managed instance selection."
  type        = number
  default     = 2
}

variable "instance_vcpu_max" {
  description = "Maximum vCPU count for managed instance selection."
  type        = number
  default     = 2
}

variable "instance_memory_mib_min" {
  description = "Minimum memory for managed instance selection."
  type        = number
  default     = 4096
}

variable "instance_memory_mib_max" {
  description = "Maximum memory for managed instance selection."
  type        = number
  default     = 4096
}

variable "container_cpu" {
  description = "CPU units reserved by the WebDAV task."
  type        = number
  default     = 512
}

variable "container_memory" {
  description = "Memory in MiB reserved by the WebDAV task."
  type        = number
  default     = 1024
}

variable "project_id" {
  description = "The GCP project ID"
  type        = string
  default     = "solana-trading-bot-462914"
}

variable "region" {
  description = "The GCP region for resources"
  type        = string
  default     = "us-central1"
}

variable "environment" {
  description = "Environment name (e.g., production, staging)"
  type        = string
  default     = "production"
}

variable "app_name" {
  description = "Application name"
  type        = string
  default     = "stb"
}

variable "image_tag" {
  description = "Docker image tag to deploy"
  type        = string
  default     = "latest"
}

variable "github_repository" {
  description = "GitHub repository in format 'owner/repo'"
  type        = string
  default     = "https://github.com/SuzukiTakamasa/solana-trading-bot"
}

variable "solana_rpc_url" {
  description = "Solana RPC endpoint URL"
  type        = string
  default     = "https://api.mainnet-beta.solana.com"
  sensitive   = true
}

variable "jupiter_api_url" {
  description = "Jupiter DEX API endpoint URL"
  type        = string
  default     = "https://quote-api.jup.ag/v6"
}

variable "slippage_bps" {
  description = "Trading slippage tolerance in basis points"
  type        = number
  default     = 50
}

variable "data_retention_days" {
  description = "Number of days to retain trading data"
  type        = number
  default     = 30
}

variable "cloud_run_memory" {
  description = "Memory allocation for Cloud Run service"
  type        = string
  default     = "1Gi"
}

variable "cloud_run_cpu" {
  description = "CPU allocation for Cloud Run service"
  type        = string
  default     = "1"
}

variable "cloud_run_timeout" {
  description = "Timeout for Cloud Run service in seconds"
  type        = number
  default     = 900
}

variable "cloud_run_max_instances" {
  description = "Maximum number of Cloud Run instances"
  type        = number
  default     = 1
}

variable "cloud_run_min_instances" {
  description = "Minimum number of Cloud Run instances"
  type        = number
  default     = 0
}

variable "scheduler_cron" {
  description = "Cron schedule for trading bot execution"
  type        = string
  default     = "0 * * * *"
}

variable "enable_monitoring" {
  description = "Enable monitoring and alerting"
  type        = bool
  default     = true
}

variable "alert_email" {
  description = "Email address for monitoring alerts"
  type        = string
  default     = ""
}

variable "labels" {
  description = "Common labels to apply to all resources"
  type        = map(string)
  default     = {}
}

variable "docker_build_context" {
  description = "The Docker build context path"
  type        = string
  default     = "../"
}

variable "enable_cloud_build" {
  description = "Enable Cloud Build trigger for automated builds"
  type        = bool
  default     = false
}

variable "trigger_branch" {
  description = "Branch to trigger Cloud Build on"
  type        = string
  default     = "main"
}
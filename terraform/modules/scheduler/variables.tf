variable "project_id" {
  description = "The GCP project ID"
  type        = string
}

variable "region" {
  description = "The GCP region"
  type        = string
}

variable "environment" {
  description = "Environment name"
  type        = string
}

variable "app_name" {
  description = "Application name"
  type        = string
}

variable "job_name" {
  description = "Cloud Scheduler job name"
  type        = string
}

variable "schedule" {
  description = "Cron schedule expression"
  type        = string
}

variable "cloud_run_service_url" {
  description = "URL of the Cloud Run service to invoke"
  type        = string
}

variable "service_account_email" {
  description = "Service account email for scheduler"
  type        = string
}

variable "labels" {
  description = "Labels to apply to resources"
  type        = map(string)
  default     = {}
}
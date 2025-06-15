variable "project_id" {
  description = "The GCP project ID"
  type        = string
}

variable "region" {
  description = "The GCP region"
  type        = string
}

variable "app_name" {
  description = "The application name"
  type        = string
}

variable "repository_name" {
  description = "The Artifact Registry repository name"
  type        = string
}

variable "image_name" {
  description = "The Docker image name"
  type        = string
}

variable "image_tag" {
  description = "The Docker image tag"
  type        = string
  default     = "latest"
}

variable "docker_build_context" {
  description = "The Docker build context path"
  type        = string
  default     = "."
}

variable "service_account_email" {
  description = "Service account email for Cloud Build"
  type        = string
}

variable "enable_cloud_build" {
  description = "Enable Cloud Build trigger"
  type        = bool
  default     = false
}

variable "github_owner" {
  description = "GitHub repository owner"
  type        = string
  default     = ""
}

variable "github_repo" {
  description = "GitHub repository name"
  type        = string
  default     = ""
}

variable "trigger_branch" {
  description = "Branch to trigger builds on"
  type        = string
  default     = "main"
}

variable "labels" {
  description = "Labels to apply to resources"
  type        = map(string)
  default     = {}
}
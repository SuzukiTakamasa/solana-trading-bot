output "cloud_run_service_url" {
  description = "The URL of the Cloud Run service"
  value       = module.cloud_run.service_url
}

output "cloud_run_service_name" {
  description = "The name of the Cloud Run service"
  value       = module.cloud_run.service_name
}

output "cloud_run_service_account_email" {
  description = "The email of the Cloud Run service account"
  value       = module.service_accounts.cloud_run_service_account_email
}

output "scheduler_job_name" {
  description = "The name of the Cloud Scheduler job"
  value       = module.scheduler.job_name
}

output "firestore_database_name" {
  description = "The name of the Firestore database"
  value       = module.firestore.database_name
}

output "secret_names" {
  description = "The names of created secrets"
  value = {
    wallet_private_key = module.secret_manager.wallet_private_key_secret_name
    line_channel_token = module.secret_manager.line_channel_token_secret_name
    line_user_id       = module.secret_manager.line_user_id_secret_name
  }
  sensitive = true
}

output "monitoring_enabled" {
  description = "Whether monitoring is enabled"
  value       = var.enable_monitoring
}

output "terraform_state_bucket" {
  description = "GCS bucket for Terraform state"
  value       = "${var.project_id}-terraform-state"
}

# Workload Identity Federation outputs for GitHub Actions
output "workload_identity_provider" {
  description = "Workload Identity Provider for GitHub Actions"
  value       = google_iam_workload_identity_pool_provider.github.name
}

output "github_actions_service_account" {
  description = "Service account email for GitHub Actions"
  value       = google_service_account.github_actions.email
}

output "artifact_registry_repository" {
  description = "Artifact Registry repository name"
  value       = module.docker_build.repository_name
}

output "docker_image_uri" {
  description = "Full Docker image URI"
  value       = module.docker_build.image_uri
}
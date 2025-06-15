output "cloud_run_service_account_email" {
  description = "Email of the Cloud Run service account"
  value       = google_service_account.cloud_run.email
}

output "cloud_run_service_account_id" {
  description = "ID of the Cloud Run service account"
  value       = google_service_account.cloud_run.unique_id
}

output "scheduler_service_account_email" {
  description = "Email of the Cloud Scheduler service account"
  value       = google_service_account.scheduler.email
}

output "scheduler_service_account_id" {
  description = "ID of the Cloud Scheduler service account"
  value       = google_service_account.scheduler.unique_id
}

output "github_actions_service_account_email" {
  description = "Email of the GitHub Actions service account"
  value       = google_service_account.github_actions.email
}

output "github_actions_service_account_id" {
  description = "ID of the GitHub Actions service account"
  value       = google_service_account.github_actions.unique_id
}
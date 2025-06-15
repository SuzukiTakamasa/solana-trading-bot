output "service_url" {
  description = "The URL of the Cloud Run service"
  value       = google_cloud_run_v2_service.app.uri
}

output "service_name" {
  description = "The name of the Cloud Run service"
  value       = google_cloud_run_v2_service.app.name
}

output "service_id" {
  description = "The ID of the Cloud Run service"
  value       = google_cloud_run_v2_service.app.id
}

output "latest_revision" {
  description = "The name of the latest revision"
  value       = google_cloud_run_v2_service.app.latest_created_revision
}

output "artifact_registry_repository" {
  description = "The Artifact Registry repository name"
  value       = google_artifact_registry_repository.app_repo.name
}
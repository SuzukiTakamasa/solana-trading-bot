output "repository_id" {
  description = "The Artifact Registry repository ID"
  value       = google_artifact_registry_repository.registry.id
}

output "repository_name" {
  description = "The Artifact Registry repository name"
  value       = google_artifact_registry_repository.registry.name
}

output "image_uri" {
  description = "The full Docker image URI"
  value       = local.image_uri
}

output "image_uri_with_tag" {
  description = "The full Docker image URI with tag"
  value       = "${local.image_uri}:${var.image_tag}"
}
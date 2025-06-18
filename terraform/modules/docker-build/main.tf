terraform {
  required_providers {
    google = {
      source  = "hashicorp/google"
      version = ">= 5.0"
    }
    docker = {
      source  = "kreuzwerker/docker"
      version = ">= 3.0"
    }
  }
}

locals {
  image_uri = "${var.region}-docker.pkg.dev/${var.project_id}/${google_artifact_registry_repository.registry.name}/${var.image_name}"
}

resource "google_artifact_registry_repository" "registry" {
  location      = var.region
  repository_id = var.repository_name
  description   = "Docker repository for ${var.app_name}"
  format        = "DOCKER"

  labels = var.labels
}

resource "google_artifact_registry_repository_iam_member" "cloud_build_writer" {
  location   = google_artifact_registry_repository.registry.location
  repository = google_artifact_registry_repository.registry.name
  role       = "roles/artifactregistry.writer"
  member     = "serviceAccount:${var.service_account_email}"
}

resource "null_resource" "docker_build_push" {
  triggers = {
    always_run = timestamp()
  }

  provisioner "local-exec" {
    command = <<-EOT
      # Authenticate with gcloud
      gcloud auth configure-docker ${var.region}-docker.pkg.dev --quiet
      
      # Build Docker image
      docker build -t ${local.image_uri}:${var.image_tag} ${var.docker_build_context}
      
      # Tag as latest
      docker tag ${local.image_uri}:${var.image_tag} ${local.image_uri}:latest
      
      # Push to Artifact Registry
      docker push ${local.image_uri}:${var.image_tag}
      docker push ${local.image_uri}:latest
    EOT

    environment = {
      DOCKER_BUILDKIT = "1"
    }
  }

  depends_on = [
    google_artifact_registry_repository.registry,
    google_artifact_registry_repository_iam_member.cloud_build_writer
  ]
}

resource "google_cloudbuild_trigger" "docker_build" {
  count = var.enable_cloud_build ? 1 : 0

  name        = "${var.app_name}-docker-build"
  description = "Build and push Docker image for ${var.app_name}"

  github {
    owner = var.github_owner
    name  = var.github_repo

    push {
      branch = var.trigger_branch
    }
  }

  build {
    step {
      name = "gcr.io/cloud-builders/docker"
      args = [
        "build",
        "-t", "${local.image_uri}:$COMMIT_SHA",
        "-t", "${local.image_uri}:latest",
        "."
      ]
    }

    step {
      name = "gcr.io/cloud-builders/docker"
      args = ["push", "--all-tags", local.image_uri]
    }

    options {
      logging = "CLOUD_LOGGING_ONLY"
    }
  }

  service_account = var.service_account_email
}
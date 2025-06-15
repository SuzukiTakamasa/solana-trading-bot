resource "google_cloud_scheduler_job" "trading_bot" {
  name        = var.job_name
  project     = var.project_id
  region      = var.region
  description = "Triggers ${var.app_name} trading bot execution"
  schedule    = var.schedule
  time_zone   = "UTC"

  retry_config {
    retry_count          = 1
    max_retry_duration   = "15s"
    min_backoff_duration = "5s"
    max_backoff_duration = "10s"
    max_doublings        = 2
  }

  attempt_deadline = "540s"

  http_target {
    http_method = "GET"
    uri         = "${var.cloud_run_service_url}/trigger"

    oidc_token {
      service_account_email = var.service_account_email
      audience              = var.cloud_run_service_url
    }
  }

  lifecycle {
    ignore_changes = [
      http_target[0].oidc_token[0].audience
    ]
  }
}

resource "google_project_iam_member" "scheduler_run_invoker" {
  project = var.project_id
  role    = "roles/run.invoker"
  member  = "serviceAccount:${var.service_account_email}"
}

resource "google_project_iam_member" "scheduler_token_creator" {
  project = var.project_id
  role    = "roles/iam.serviceAccountTokenCreator"
  member  = "serviceAccount:${var.service_account_email}"
}
resource "google_secret_manager_secret" "wallet_private_key" {
  secret_id = "${var.app_name}-wallet-private-key-${var.environment}"
  project   = var.project_id

  replication {
    auto {}
  }

  labels = var.labels
}

resource "google_secret_manager_secret" "line_channel_token" {
  secret_id = "${var.app_name}-line-channel-token-${var.environment}"
  project   = var.project_id

  replication {
    auto {}
  }

  labels = var.labels
}

resource "google_secret_manager_secret" "line_user_id" {
  secret_id = "${var.app_name}-line-user-id-${var.environment}"
  project   = var.project_id

  replication {
    auto {}
  }

  labels = var.labels
}

resource "google_secret_manager_secret_iam_member" "wallet_key_accessor" {
  project   = var.project_id
  secret_id = google_secret_manager_secret.wallet_private_key.secret_id
  role      = "roles/secretmanager.secretAccessor"
  member    = "serviceAccount:${var.service_account_email}"
}

resource "google_secret_manager_secret_iam_member" "line_token_accessor" {
  project   = var.project_id
  secret_id = google_secret_manager_secret.line_channel_token.secret_id
  role      = "roles/secretmanager.secretAccessor"
  member    = "serviceAccount:${var.service_account_email}"
}

resource "google_secret_manager_secret_iam_member" "line_user_accessor" {
  project   = var.project_id
  secret_id = google_secret_manager_secret.line_user_id.secret_id
  role      = "roles/secretmanager.secretAccessor"
  member    = "serviceAccount:${var.service_account_email}"
}

resource "google_project_iam_member" "secret_accessor" {
  project = var.project_id
  role    = "roles/secretmanager.viewer"
  member  = "serviceAccount:${var.service_account_email}"
}
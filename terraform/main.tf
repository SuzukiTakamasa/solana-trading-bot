locals {
  common_labels = merge(
    {
      environment = var.environment
      app         = var.app_name
      managed_by  = "terraform"
    },
    var.labels
  )
  
  service_name        = "${var.app_name}-${var.environment}"
  service_account_id  = "${var.app_name}-sa-${var.environment}"
  scheduler_job_name  = "${var.app_name}-scheduler-${var.environment}"
}

data "google_project" "project" {
  project_id = var.project_id
}

resource "google_project_service" "required_apis" {
  for_each = toset([
    "artifactregistry.googleapis.com",
    "run.googleapis.com",
    "firestore.googleapis.com",
    "cloudscheduler.googleapis.com",
    "secretmanager.googleapis.com",
    "logging.googleapis.com",
    "monitoring.googleapis.com",
    "cloudresourcemanager.googleapis.com",
    "iam.googleapis.com",
    "iamcredentials.googleapis.com",
    "sts.googleapis.com",
    "cloudbuild.googleapis.com",
  ])

  project = var.project_id
  service = each.value

  disable_on_destroy = false
}

module "service_accounts" {
  source = "./modules/service-accounts"

  project_id         = var.project_id
  environment        = var.environment
  app_name          = var.app_name
  service_account_id = local.service_account_id
  labels            = local.common_labels

  depends_on = [google_project_service.required_apis]
}

# Workload Identity Federation for GitHub Actions
resource "google_iam_workload_identity_pool" "github" {
  provider = google-beta
  
  workload_identity_pool_id = "github-actions-pool"
  display_name              = "GitHub Actions Pool"
  description              = "Workload Identity Pool for GitHub Actions"
  disabled                 = false
  
  project = var.project_id
}

resource "google_iam_workload_identity_pool_provider" "github" {
  provider = google-beta
  
  workload_identity_pool_id          = google_iam_workload_identity_pool.github.workload_identity_pool_id
  workload_identity_pool_provider_id = "github-actions-provider"
  display_name                       = "GitHub Actions Provider"
  description                       = "OIDC provider for GitHub Actions"
  
  attribute_mapping = {
    "google.subject"       = "assertion.sub"
    "attribute.actor"      = "assertion.actor"
    "attribute.repository" = "assertion.repository"
    "attribute.repository_owner" = "assertion.repository_owner"
  }
  
  attribute_condition = "assertion.repository == '${var.github_repository}'"
  
  oidc {
    issuer_uri = "https://token.actions.githubusercontent.com"
  }
  
  project = var.project_id
}

# Service account for GitHub Actions
resource "google_service_account" "github_actions" {
  account_id   = "${var.app_name}-gh-actions"
  display_name = "GitHub Actions Service Account"
  description  = "Service account for GitHub Actions CI/CD"
  project      = var.project_id
}

# Allow GitHub Actions to impersonate the service account
resource "google_service_account_iam_member" "github_actions_impersonation" {
  service_account_id = google_service_account.github_actions.id
  role               = "roles/iam.workloadIdentityUser"
  member             = "principalSet://iam.googleapis.com/${google_iam_workload_identity_pool.github.name}/attribute.repository/${var.github_repository}"
}

# Grant necessary permissions to GitHub Actions service account
resource "google_project_iam_member" "github_actions_permissions" {
  for_each = toset([
    "roles/run.admin",
    "roles/iam.serviceAccountUser",
    "roles/storage.admin",
    "roles/artifactregistry.admin",
    "roles/secretmanager.admin",
    "roles/cloudscheduler.admin",
    "roles/logging.viewer",
    "roles/monitoring.viewer"
  ])
  
  project = var.project_id
  role    = each.value
  member  = "serviceAccount:${google_service_account.github_actions.email}"
}

module "secret_manager" {
  source = "./modules/secret-manager"

  project_id     = var.project_id
  environment    = var.environment
  app_name       = var.app_name
  labels         = local.common_labels
  
  service_account_email = module.service_accounts.cloud_run_service_account_email

  depends_on = [
    google_project_service.required_apis,
    module.service_accounts
  ]
}

module "docker_build" {
  source = "./modules/docker-build"

  project_id            = var.project_id
  region                = var.region
  app_name              = var.app_name
  repository_name       = var.app_name
  image_name            = var.app_name
  image_tag             = var.image_tag
  docker_build_context  = var.docker_build_context
  service_account_email = google_service_account.github_actions.email
  enable_cloud_build    = var.enable_cloud_build
  github_owner          = split("/", var.github_repository)[0]
  github_repo           = split("/", var.github_repository)[1]
  trigger_branch        = var.trigger_branch
  labels                = local.common_labels

  depends_on = [
    google_project_service.required_apis,
    google_service_account.github_actions
  ]
}

module "firestore" {
  source = "./modules/firestore"

  project_id  = var.project_id
  environment = var.environment
  app_name    = var.app_name
  labels      = local.common_labels
  
  service_account_email = module.service_accounts.cloud_run_service_account_email

  depends_on = [google_project_service.required_apis]
}

module "cloud_run" {
  source = "./modules/cloud-run"

  project_id              = var.project_id
  region                  = var.region
  environment             = var.environment
  app_name                = var.app_name
  service_name            = local.service_name
  image_tag               = var.image_tag
  
  service_account_email   = module.service_accounts.cloud_run_service_account_email
  
  memory                  = var.cloud_run_memory
  cpu                     = var.cloud_run_cpu
  timeout                 = var.cloud_run_timeout
  max_instances           = var.cloud_run_max_instances
  min_instances           = var.cloud_run_min_instances
  
  env_vars = {
    SOLANA_RPC_URL       = var.solana_rpc_url
    JUPITER_API_URL      = var.jupiter_api_url
    SLIPPAGE_BPS         = tostring(var.slippage_bps)
    GCP_PROJECT_ID       = var.project_id
    DATA_RETENTION_DAYS  = tostring(var.data_retention_days)
    CLOUD_RUN_SA_EMAIL   = module.service_accounts.cloud_run_service_account_email
  }
  
  secret_env_vars = {
    WALLET_PRIVATE_KEY   = module.secret_manager.wallet_private_key_secret_name
    LINE_CHANNEL_TOKEN   = module.secret_manager.line_channel_token_secret_name
    LINE_USER_ID         = module.secret_manager.line_user_id_secret_name
  }
  
  labels = local.common_labels

  depends_on = [
    google_project_service.required_apis,
    module.service_accounts,
    module.secret_manager
  ]
}

module "scheduler" {
  source = "./modules/scheduler"

  project_id             = var.project_id
  region                 = var.region
  environment            = var.environment
  app_name               = var.app_name
  job_name               = local.scheduler_job_name
  schedule               = var.scheduler_cron
  
  cloud_run_service_url  = module.cloud_run.service_url
  service_account_email  = module.service_accounts.scheduler_service_account_email
  
  labels = local.common_labels

  depends_on = [
    google_project_service.required_apis,
    module.cloud_run,
    module.service_accounts
  ]
}

resource "google_monitoring_notification_channel" "email" {
  count = var.enable_monitoring && var.alert_email != "" ? 1 : 0

  display_name = "${local.service_name}-alerts"
  type         = "email"

  labels = {
    email_address = var.alert_email
  }

  user_labels = local.common_labels
}

resource "google_monitoring_alert_policy" "cloud_run_errors" {
  count = var.enable_monitoring ? 1 : 0

  display_name = "${local.service_name} - High Error Rate"
  combiner     = "OR"

  conditions {
    display_name = "Error rate above 5%"

    condition_threshold {
      filter          = "resource.type=\"cloud_run_revision\" AND resource.labels.service_name=\"${local.service_name}\" AND metric.type=\"run.googleapis.com/request_count\""
      duration        = "300s"
      comparison      = "COMPARISON_GT"
      threshold_value = 0.05

      aggregations {
        alignment_period     = "60s"
        per_series_aligner   = "ALIGN_RATE"
        cross_series_reducer = "REDUCE_SUM"
        group_by_fields      = ["resource.labels.service_name"]
      }

      trigger {
        count = 1
      }
    }
  }

  notification_channels = var.enable_monitoring && var.alert_email != "" ? [google_monitoring_notification_channel.email[0].id] : []
  
  user_labels = local.common_labels

  alert_strategy {
    auto_close = "1800s"
  }
}

resource "google_logging_metric" "trading_errors" {
  count = var.enable_monitoring ? 1 : 0

  name        = "${var.app_name}_trading_errors"
  description = "Count of trading errors"
  
  filter = <<-EOT
    resource.type="cloud_run_revision"
    resource.labels.service_name="${local.service_name}"
    severity>=ERROR
    jsonPayload.message=~"trading error|trade failed|insufficient balance"
  EOT

  metric_descriptor {
    metric_kind = "DELTA"
    value_type  = "INT64"
    unit        = "1"
    
    labels {
      key         = "error_type"
      value_type  = "STRING"
      description = "Type of trading error"
    }
  }

  label_extractors = {
    "error_type" = "EXTRACT(jsonPayload.error_type)"
  }
}
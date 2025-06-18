locals {
  database_id = "(default)"
}

resource "google_firestore_database" "database" {
  project                     = var.project_id
  name                        = local.database_id
  location_id                 = "nam5"
  type                        = "FIRESTORE_NATIVE"
  concurrency_mode            = "OPTIMISTIC"
  app_engine_integration_mode = "DISABLED"

  lifecycle {
    prevent_destroy = true
  }
}

resource "google_firestore_index" "price_history_timestamp" {
  project    = var.project_id
  database   = google_firestore_database.database.name
  collection = "price_history"

  fields {
    field_path = "timestamp"
    order      = "DESCENDING"
  }

  fields {
    field_path = "data_source"
    order      = "ASCENDING"
  }

  fields {
    field_path = "__name__"
    order      = "DESCENDING"
  }
}

resource "google_firestore_index" "trading_sessions_timestamp" {
  project    = var.project_id
  database   = google_firestore_database.database.name
  collection = "trading_sessions"

  fields {
    field_path = "timestamp"
    order      = "DESCENDING"
  }

  fields {
    field_path = "action"
    order      = "ASCENDING"
  }

  fields {
    field_path = "__name__"
    order      = "DESCENDING"
  }
}

resource "google_firestore_index" "profit_tracking_timestamp" {
  project    = var.project_id
  database   = google_firestore_database.database.name
  collection = "profit_tracking"

  fields {
    field_path = "timestamp"
    order      = "DESCENDING"
  }

  fields {
    field_path = "session_id"
    order      = "ASCENDING"
  }

  fields {
    field_path = "__name__"
    order      = "DESCENDING"
  }
}

resource "google_firestore_index" "trading_sessions_success" {
  project    = var.project_id
  database   = google_firestore_database.database.name
  collection = "trading_sessions"

  fields {
    field_path = "success"
    order      = "ASCENDING"
  }

  fields {
    field_path = "timestamp"
    order      = "DESCENDING"
  }

  fields {
    field_path = "__name__"
    order      = "DESCENDING"
  }
}

resource "google_project_iam_member" "firestore_user" {
  project = var.project_id
  role    = "roles/datastore.user"
  member  = "serviceAccount:${var.service_account_email}"
}

resource "google_firestore_document" "security_rules_placeholder" {
  project     = var.project_id
  database    = google_firestore_database.database.name
  collection  = "_terraform"
  document_id = "placeholder"

  fields = jsonencode({
    created_at = {
      timestampValue = timestamp()
    }
    description = {
      stringValue = "Placeholder document for Terraform management"
    }
    environment = {
      stringValue = var.environment
    }
  })

  lifecycle {
    ignore_changes = [fields]
  }
}
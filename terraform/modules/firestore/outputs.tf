output "database_name" {
  description = "The name of the Firestore database"
  value       = google_firestore_database.database.name
}

output "database_location" {
  description = "The location of the Firestore database"
  value       = google_firestore_database.database.location_id
}

output "indexes_created" {
  description = "List of created Firestore indexes"
  value = [
    google_firestore_index.price_history_timestamp.name,
    google_firestore_index.trading_sessions_timestamp.name,
    google_firestore_index.profit_tracking_timestamp.name,
    google_firestore_index.trading_sessions_success.name
  ]
}
output "wallet_private_key_secret_name" {
  description = "The name of the wallet private key secret"
  value       = google_secret_manager_secret.wallet_private_key.secret_id
}

output "line_channel_token_secret_name" {
  description = "The name of the LINE channel token secret"
  value       = google_secret_manager_secret.line_channel_token.secret_id
}

output "line_user_id_secret_name" {
  description = "The name of the LINE user ID secret"
  value       = google_secret_manager_secret.line_user_id.secret_id
}

output "secret_ids" {
  description = "Map of all secret IDs"
  value = {
    wallet_private_key = google_secret_manager_secret.wallet_private_key.id
    line_channel_token = google_secret_manager_secret.line_channel_token.id
    line_user_id       = google_secret_manager_secret.line_user_id.id
  }
}
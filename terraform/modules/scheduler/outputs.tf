output "job_name" {
  description = "The name of the Cloud Scheduler job"
  value       = google_cloud_scheduler_job.trading_bot.name
}

output "job_id" {
  description = "The ID of the Cloud Scheduler job"
  value       = google_cloud_scheduler_job.trading_bot.id
}

output "schedule" {
  description = "The cron schedule of the job"
  value       = google_cloud_scheduler_job.trading_bot.schedule
}

output "next_scheduled_time" {
  description = "The next scheduled execution time"
  value       = google_cloud_scheduler_job.trading_bot.state
}
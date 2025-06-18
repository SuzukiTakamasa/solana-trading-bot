project_id       = "solana-trading-bot-462914"
region           = "us-central1"
environment      = "production"
github_repository = "owner/solana-trading-bot"

solana_rpc_url   = "https://api.mainnet-beta.solana.com"
jupiter_api_url  = "https://quote-api.jup.ag/v6"

slippage_bps         = 50
data_retention_days  = 30

cloud_run_memory        = "1Gi"
cloud_run_cpu          = "1"
cloud_run_timeout      = 900
cloud_run_max_instances = 1
cloud_run_min_instances = 0

scheduler_cron = "0 * * * *"

enable_monitoring = true
alert_email      = "incubus.appalachia@gmail.com"

labels = {
  team        = "trading"
  cost_center = "engineering"
}
# Solana Trading Bot

An automated SOL-USDC trading bot that runs on Google Cloud Platform with Jupiter DEX integration.

## Features

- **Automated Trading**: Converts SOL to USDC initially, then monitors prices hourly
- **Jupiter DEX Integration**: Uses Jupiter aggregator for best swap rates
- **Price Monitoring**: Checks SOL/USDC prices every hour and trades based on price movements
- **LINE Notifications**: Sends trade notifications and profit reports via LINE
- **Cloud Run Deployment**: Runs serverlessly on GCP with automatic scaling
- **GitHub Actions CI/CD**: Automated deployment pipeline

## Architecture

```
┌─────────────────┐     ┌──────────────┐     ┌─────────────┐
│  Cloud Scheduler│────▶│  Cloud Run   │────▶│  Solana     │
│  (Hourly Cron)  │     │(Trading Bot) │     │  Mainnet    │
└─────────────────┘     └──────────────┘     └─────────────┘
                               │                      │
                               ▼                      ▼
                        ┌──────────────┐     ┌─────────────┐
                        │  LINE Bot    │     │ Jupiter DEX │
                        │ Notifications│     │    API      │
                        └──────────────┘     └─────────────┘
```

## Prerequisites

1. **Solana Wallet**: A Phantom wallet with SOL balance
2. **GCP Account**: For Cloud Run and Cloud Scheduler
3. **LINE Developer Account**: For bot notifications
4. **GitHub Account**: For repository and Actions

## Setup Instructions

### 1. Clone the Repository

```bash
git clone https://github.com/yourusername/solana-trading-bot.git
cd solana-trading-bot
```

### 2. Configure Environment Variables

Copy `.env.example` to `.env` and fill in your values:

```bash
cp .env.example .env
```

Required variables:
- `WALLET_PRIVATE_KEY`: Your Solana wallet private key (base58 format)
- `LINE_CHANNEL_TOKEN`: LINE Messaging API channel access token
- `LINE_USER_ID`: Your LINE user ID for receiving notifications

### 3. Set up GCP

1. Create a new GCP project
2. Enable required APIs:
   ```bash
   gcloud services enable run.googleapis.com
   gcloud services enable containerregistry.googleapis.com
   gcloud services enable cloudscheduler.googleapis.com
   ```

3. Create a service account:
   ```bash
   gcloud iam service-accounts create github-actions \
     --display-name="GitHub Actions"
   
   gcloud projects add-iam-policy-binding PROJECT_ID \
     --member="serviceAccount:github-actions@PROJECT_ID.iam.gserviceaccount.com" \
     --role="roles/run.admin"
   
   gcloud projects add-iam-policy-binding PROJECT_ID \
     --member="serviceAccount:github-actions@PROJECT_ID.iam.gserviceaccount.com" \
     --role="roles/storage.admin"
   
   gcloud projects add-iam-policy-binding PROJECT_ID \
     --member="serviceAccount:github-actions@PROJECT_ID.iam.gserviceaccount.com" \
     --role="roles/cloudscheduler.admin"
   ```

4. Create and download service account key:
   ```bash
   gcloud iam service-accounts keys create key.json \
     --iam-account=github-actions@PROJECT_ID.iam.gserviceaccount.com
   ```

### 4. Configure GitHub Secrets

Add these secrets to your GitHub repository:

- `GCP_PROJECT_ID`: Your GCP project ID
- `GCP_SA_KEY`: Contents of the service account key JSON
- `SOLANA_RPC_URL`: Solana RPC endpoint (optional, defaults to mainnet)
- `WALLET_PRIVATE_KEY`: Your wallet private key
- `LINE_CHANNEL_TOKEN`: LINE channel access token
- `LINE_USER_ID`: LINE user ID
- `JUPITER_API_URL`: Jupiter API URL (optional)
- `SLIPPAGE_BPS`: Slippage in basis points (optional, default 50)

### 5. Deploy

#### Option A: GitHub Actions (Recommended)

Push to the main branch to trigger automatic deployment:

```bash
git add .
git commit -m "Initial deployment"
git push origin main
```

#### Option B: Manual Deployment

```bash
# Set environment variables
export GCP_PROJECT_ID=your-project-id
export GCP_REGION=us-central1

# Run deployment script
./deploy.sh
```

## Trading Strategy

The bot implements a simple momentum-based strategy:

1. **Initial State**: Converts all SOL to USDC
2. **Hourly Checks**: 
   - If SOL price increased >0.5%: Swap USDC → SOL
   - If USDC price increased >0.5%: Swap SOL → USDC
3. **Notifications**: Sends LINE message on each trade with profit info

## Monitoring

### View Logs

```bash
gcloud run services logs read solana-trading-bot --region=us-central1
```

### Check Service Status

```bash
gcloud run services describe solana-trading-bot --region=us-central1
```

### View Scheduled Jobs

```bash
gcloud scheduler jobs list --location=us-central1
```

## Local Development

### Build and Run

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build
cargo build

# Run with environment variables
cargo run
```

### Docker Build

```bash
docker build -t solana-trading-bot .
docker run -p 8080:8080 --env-file .env solana-trading-bot
```

## Security Considerations

1. **Private Keys**: Never commit private keys to the repository
2. **Service Account**: Use least-privilege principles for GCP service accounts
3. **Environment Variables**: Use GitHub Secrets and Cloud Run secrets
4. **API Keys**: Rotate LINE tokens regularly
5. **Wallet Security**: Use a dedicated trading wallet with limited funds

## Customization

### Modify Trading Strategy

Edit `src/trading.rs` to implement your own trading logic:

```rust
fn should_swap_to_sol(current_sol_price: Decimal) -> bool {
    // Implement your strategy here
}
```

### Adjust Trading Frequency

Modify the Cloud Scheduler cron expression in `deploy.sh`:

```bash
--schedule "0 * * * *"  # Every hour
--schedule "*/30 * * * *"  # Every 30 minutes
--schedule "0 */4 * * *"  # Every 4 hours
```

## Troubleshooting

### Common Issues

1. **Transaction Failures**: Check wallet balance and RPC endpoint
2. **LINE Notifications Not Working**: Verify channel token and user ID
3. **Deployment Failures**: Check GCP quotas and permissions
4. **Price Feed Issues**: Ensure Jupiter API is accessible

### Debug Mode

Enable debug logging:

```bash
export RUST_LOG=solana_trading_bot=debug
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Commit your changes
4. Push to the branch
5. Create a Pull Request

## License

MIT License - see LICENSE file for details

## Disclaimer

This bot is for educational purposes only. Cryptocurrency trading carries significant risk. Always test thoroughly with small amounts before using in production. The authors are not responsible for any financial losses.
# Terraform Infrastructure Management

This directory contains the Infrastructure as Code (IaC) configuration for the Solana Trading Bot using Terraform and Google Cloud Platform (GCP).

## Architecture Overview

The Terraform configuration manages the following GCP resources:

- **Cloud Run**: Serverless container hosting for the trading bot
- **Cloud Firestore**: NoSQL database for storing trading data
- **Cloud Scheduler**: Cron job for hourly bot execution
- **Secret Manager**: Secure storage for sensitive configuration
- **Service Accounts**: IAM roles with least-privilege access
- **Artifact Registry**: Docker image storage
- **Monitoring & Alerting**: Error tracking and notifications

## Prerequisites

1. **GCP Project** with billing enabled
2. **Terraform** v1.5.0 or higher
3. **gcloud CLI** installed and configured
4. **GitHub repository secrets** configured:
   - `GCP_PROJECT_ID`: Your GCP project ID
   - `WIF_PROVIDER`: Workload Identity Federation provider
   - `WIF_SERVICE_ACCOUNT`: Service account for WIF
   - `SOLANA_RPC_URL`: Solana RPC endpoint
   - `WALLET_PRIVATE_KEY`: Trading wallet private key
   - `LINE_CHANNEL_TOKEN`: LINE messaging API token
   - `LINE_USER_ID`: LINE user ID for notifications
   - `ALERT_EMAIL`: Email for monitoring alerts (optional)

## Initial Setup

### 1. Set up Terraform Backend

First, create a GCS bucket for Terraform state:

```bash
export GCP_PROJECT_ID="your-project-id"
export GCP_REGION="us-central1"

# Run the setup script
./scripts/setup-terraform-backend.sh
```

### 2. Configure Terraform Variables

Create a `terraform.tfvars` file based on the example:

```bash
cd terraform
cp terraform.tfvars.example terraform.tfvars
```

Edit `terraform.tfvars` with your configuration:

```hcl
project_id       = "your-gcp-project-id"
region           = "us-central1"
environment      = "production"
github_repository = "your-github-username/solana-trading-bot"

# Sensitive values should be set via environment variables or GitHub Secrets
solana_rpc_url   = "https://api.mainnet-beta.solana.com"

# Optional configurations
enable_monitoring = true
alert_email      = "your-email@example.com"
```

### 3. Initialize Terraform

```bash
cd terraform
terraform init
```

## Deployment

### Manual Deployment

1. **Plan changes**:
   ```bash
   terraform plan
   ```

2. **Apply changes**:
   ```bash
   terraform apply
   ```

3. **View outputs**:
   ```bash
   terraform output
   ```

### Automated Deployment via GitHub Actions

The infrastructure is automatically managed through GitHub Actions:

1. **On Pull Request**: Runs `terraform plan` and posts results as PR comment
2. **On merge to main**: Automatically applies Terraform changes
3. **Manual trigger**: Use the workflow dispatch to run plan/apply/destroy

## Managing Secrets

After infrastructure is created, populate the secrets:

```bash
# Set secrets using gcloud CLI
echo -n "your-wallet-private-key" | gcloud secrets versions add \
  solana-trading-bot-wallet-private-key-production --data-file=-

echo -n "your-line-token" | gcloud secrets versions add \
  solana-trading-bot-line-channel-token-production --data-file=-

echo -n "your-line-user-id" | gcloud secrets versions add \
  solana-trading-bot-line-user-id-production --data-file=-
```

## Module Structure

```
terraform/
├── modules/
│   ├── cloud-run/          # Cloud Run service configuration
│   ├── firestore/          # Firestore database and indexes
│   ├── scheduler/          # Cloud Scheduler jobs
│   ├── secret-manager/     # Secret Manager resources
│   └── service-accounts/   # IAM service accounts
├── main.tf                 # Main configuration
├── variables.tf            # Input variables
├── outputs.tf              # Output values
└── versions.tf             # Provider versions
```

## Cost Optimization

The infrastructure is configured for cost optimization:

- **Cloud Run**: Scales to zero when not in use
- **Firestore**: Using native mode with minimal indexes
- **State Storage**: Lifecycle rules to delete old state versions
- **Monitoring**: Only essential metrics and alerts

Estimated monthly costs (with hourly execution):
- Cloud Run: ~$5-10
- Firestore: ~$1-5
- Cloud Scheduler: Free tier
- Secret Manager: ~$0.06
- Total: ~$10-20/month

## Security Best Practices

1. **Least Privilege**: Service accounts have minimal required permissions
2. **Secret Management**: All sensitive data stored in Secret Manager
3. **State Encryption**: Terraform state encrypted at rest in GCS
4. **Workload Identity**: GitHub Actions uses WIF instead of service account keys
5. **Network Security**: Cloud Run service is publicly accessible but validates requests

## Troubleshooting

### Common Issues

1. **Permission Denied**:
   ```bash
   # Enable required APIs
   gcloud services enable run.googleapis.com firestore.googleapis.com \
     cloudscheduler.googleapis.com secretmanager.googleapis.com
   ```

2. **State Lock**:
   ```bash
   # Force unlock if needed (use with caution)
   terraform force-unlock <lock-id>
   ```

3. **Resource Already Exists**:
   ```bash
   # Import existing resources
   terraform import module.firestore.google_firestore_database.database "(default)"
   ```

### Debugging

Enable detailed logging:
```bash
export TF_LOG=DEBUG
terraform plan
```

## Maintenance

### Regular Tasks

1. **Review costs**: Check GCP billing monthly
2. **Update dependencies**: Keep Terraform and providers updated
3. **Rotate secrets**: Rotate wallet keys and API tokens quarterly
4. **Review logs**: Check Cloud Logging for errors weekly

### Backup and Recovery

1. **State backup**: Automatic via GCS versioning
2. **Database backup**: Export Firestore data periodically
   ```bash
   gcloud firestore export gs://your-backup-bucket/firestore-backup
   ```

### Destroying Infrastructure

To tear down all resources:

```bash
# Remove data protection
terraform state rm module.firestore.google_firestore_database.database

# Destroy infrastructure
terraform destroy

# Clean up state bucket (optional)
gsutil rm -r gs://${GCP_PROJECT_ID}-terraform-state
```

## GitHub Actions Integration

### Required Secrets

Configure these in your GitHub repository settings:

1. **GCP_PROJECT_ID**: Your GCP project ID
2. **WIF_PROVIDER**: Workload Identity Federation provider
   ```
   projects/PROJECT_NUMBER/locations/global/workloadIdentityPools/github-pool/providers/github-provider
   ```
3. **WIF_SERVICE_ACCOUNT**: Service account for GitHub Actions
   ```
   solana-trading-bot-sa-github@PROJECT_ID.iam.gserviceaccount.com
   ```

### Workflow Files

- **terraform.yml**: Infrastructure management
- **deploy.yml**: Application deployment

### Setting up Workload Identity Federation

```bash
# Create workload identity pool
gcloud iam workload-identity-pools create github-pool \
  --location="global" \
  --display-name="GitHub Actions Pool"

# Create provider
gcloud iam workload-identity-pools providers create-oidc github-provider \
  --location="global" \
  --workload-identity-pool="github-pool" \
  --display-name="GitHub Provider" \
  --attribute-mapping="google.subject=assertion.sub,attribute.actor=assertion.actor,attribute.repository=assertion.repository" \
  --issuer-uri="https://token.actions.githubusercontent.com"

# Grant permissions
gcloud iam service-accounts add-iam-policy-binding \
  solana-trading-bot-sa-github@PROJECT_ID.iam.gserviceaccount.com \
  --role="roles/iam.workloadIdentityUser" \
  --member="principalSet://iam.googleapis.com/projects/PROJECT_NUMBER/locations/global/workloadIdentityPools/github-pool/attribute.repository/YOUR_GITHUB_USERNAME/solana-trading-bot"
```

## Contributing

When making infrastructure changes:

1. Create a feature branch
2. Make changes to Terraform files
3. Run `terraform fmt` to format code
4. Run `terraform validate` to check syntax
5. Create PR - GitHub Actions will run plan
6. Review plan output in PR comments
7. Merge after approval - changes auto-apply

## Support

For issues or questions:
1. Check the troubleshooting section
2. Review Cloud Logging for errors
3. Open an issue in the GitHub repository
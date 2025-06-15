# Infrastructure Quick Start Guide

This guide helps you quickly set up the Terraform infrastructure for the Solana Trading Bot.

## Prerequisites

- GCP Project with billing enabled
- `gcloud` CLI installed and authenticated
- `terraform` CLI installed (v1.5.0+)
- GitHub repository forked/cloned

## Step-by-Step Setup

### 1. Export Environment Variables

```bash
export GCP_PROJECT_ID="your-gcp-project-id"
export GCP_REGION="us-central1"
export GITHUB_REPOSITORY="your-username/solana-trading-bot"
```

### 2. Enable Required APIs

```bash
gcloud services enable \
  run.googleapis.com \
  firestore.googleapis.com \
  cloudscheduler.googleapis.com \
  secretmanager.googleapis.com \
  artifactregistry.googleapis.com \
  containerregistry.googleapis.com \
  cloudbuild.googleapis.com \
  iam.googleapis.com \
  cloudresourcemanager.googleapis.com
```

### 3. Set Up Terraform Backend

```bash
./scripts/setup-terraform-backend.sh
```

### 4. Configure Terraform

```bash
cd terraform
cp terraform.tfvars.example terraform.tfvars
# Edit terraform.tfvars with your values
```

### 5. Initialize and Apply Terraform

```bash
terraform init
terraform plan
terraform apply
```

### 6. Set Up Workload Identity for GitHub Actions

```bash
cd ..
./scripts/setup-workload-identity.sh
```

Copy the output values and add them as GitHub Secrets.

### 7. Add GitHub Secrets

Go to your GitHub repository settings and add:

- `GCP_PROJECT_ID`: Your GCP project ID
- `WIF_PROVIDER`: Output from setup script
- `WIF_SERVICE_ACCOUNT`: Output from setup script
- `SOLANA_RPC_URL`: Your Solana RPC endpoint
- `WALLET_PRIVATE_KEY`: Your trading wallet private key
- `LINE_CHANNEL_TOKEN`: LINE messaging API token
- `LINE_USER_ID`: LINE user ID
- `ALERT_EMAIL`: Email for alerts (optional)

### 8. Add Secrets to Secret Manager

```bash
# Add wallet private key
echo -n "your-wallet-private-key" | gcloud secrets versions add \
  $(terraform output -raw secret_names | jq -r '.wallet_private_key') \
  --data-file=-

# Add LINE channel token
echo -n "your-line-token" | gcloud secrets versions add \
  $(terraform output -raw secret_names | jq -r '.line_channel_token') \
  --data-file=-

# Add LINE user ID
echo -n "your-line-user-id" | gcloud secrets versions add \
  $(terraform output -raw secret_names | jq -r '.line_user_id') \
  --data-file=-
```

### 9. Deploy the Application

Push to main branch or manually trigger the deployment workflow:

```bash
git add .
git commit -m "feat: add terraform infrastructure"
git push origin main
```

## Verification

After deployment, verify everything is working:

```bash
# Check Cloud Run service
SERVICE_URL=$(cd terraform && terraform output -raw cloud_run_service_url)
curl "${SERVICE_URL}/health"

# Check scheduler job
gcloud scheduler jobs list --location="${GCP_REGION}"

# Check logs
gcloud logging read "resource.type=cloud_run_revision" --limit=10
```

## Daily Operations

### View Trading Performance

```bash
curl "${SERVICE_URL}/api/performance"
```

### Trigger Manual Trade

```bash
curl "${SERVICE_URL}/trigger"
```

### View Logs

```bash
# Recent logs
gcloud logging read "resource.type=cloud_run_revision AND severity>=INFO" \
  --limit=50 --format=json | jq -r '.[] | "\(.timestamp) [\(.severity)] \(.jsonPayload.message)"'

# Error logs only
gcloud logging read "resource.type=cloud_run_revision AND severity>=ERROR" \
  --limit=20
```

## Troubleshooting

### Infrastructure Issues

```bash
# Check Terraform state
cd terraform
terraform show

# Refresh state
terraform refresh

# Re-apply if needed
terraform apply -refresh-only
```

### Application Issues

```bash
# Check Cloud Run service
gcloud run services describe solana-trading-bot-production \
  --region="${GCP_REGION}" --format=json | jq

# Check recent revisions
gcloud run revisions list --service=solana-trading-bot-production \
  --region="${GCP_REGION}"

# Roll back if needed
gcloud run services update-traffic solana-trading-bot-production \
  --region="${GCP_REGION}" \
  --to-revisions=REVISION_NAME=100
```

## Cost Management

Monitor costs:

```bash
# Set up budget alert
gcloud billing budgets create \
  --billing-account=BILLING_ACCOUNT_ID \
  --display-name="Solana Trading Bot Budget" \
  --budget-amount=50USD \
  --threshold-rule=percent=80
```

## Cleanup

To destroy all resources:

```bash
cd terraform
terraform destroy
```

Remove the state bucket:

```bash
gsutil rm -r gs://${GCP_PROJECT_ID}-terraform-state
```
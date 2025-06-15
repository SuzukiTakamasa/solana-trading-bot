#!/bin/bash
set -euo pipefail

# Script to set up GCS backend for Terraform state management

echo "Setting up Terraform backend for Solana Trading Bot..."

# Check if required environment variables are set
if [ -z "${GCP_PROJECT_ID:-}" ]; then
    echo "Error: GCP_PROJECT_ID environment variable is not set"
    exit 1
fi

BUCKET_NAME="${GCP_PROJECT_ID}-terraform-state"
REGION="${GCP_REGION:-us-central1}"

echo "Creating GCS bucket for Terraform state: ${BUCKET_NAME}"

# Create the bucket if it doesn't exist
if ! gsutil ls -b "gs://${BUCKET_NAME}" &>/dev/null; then
    gsutil mb -p "${GCP_PROJECT_ID}" -c STANDARD -l "${REGION}" "gs://${BUCKET_NAME}"
    echo "Bucket created successfully"
else
    echo "Bucket already exists"
fi

# Enable versioning on the bucket
echo "Enabling versioning on the bucket..."
gsutil versioning set on "gs://${BUCKET_NAME}"

# Set bucket lifecycle rule to delete old versions after 30 days
echo "Setting lifecycle rules..."
cat > /tmp/lifecycle.json <<EOF
{
  "lifecycle": {
    "rule": [
      {
        "action": {"type": "Delete"},
        "condition": {
          "age": 30,
          "isLive": false
        }
      }
    ]
  }
}
EOF

gsutil lifecycle set /tmp/lifecycle.json "gs://${BUCKET_NAME}"
rm /tmp/lifecycle.json

# Enable uniform bucket-level access
echo "Enabling uniform bucket-level access..."
gsutil uniformbucketlevelaccess set on "gs://${BUCKET_NAME}"

# Create backend configuration file
echo "Creating backend configuration file..."
cat > terraform/backend.tf <<EOF
terraform {
  backend "gcs" {
    bucket = "${BUCKET_NAME}"
    prefix = "terraform/state/solana-trading-bot"
  }
}
EOF

echo "Terraform backend setup complete!"
echo ""
echo "Next steps:"
echo "1. Navigate to the terraform directory: cd terraform"
echo "2. Initialize Terraform: terraform init"
echo "3. Create your terraform.tfvars file based on terraform.tfvars.example"
echo "4. Run terraform plan to review changes"
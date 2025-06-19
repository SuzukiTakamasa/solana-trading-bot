#!/bin/bash

if [ -f .env ]; then
  set -a
  source .env
  set +a
fi

set -euo pipefail

# Script to set up Workload Identity Federation for GitHub Actions

echo "Setting up Workload Identity Federation for GitHub Actions..."

# Check if required environment variables are set
if [ -z "${GCP_PROJECT_ID:-}" ]; then
    echo "Error: GCP_PROJECT_ID environment variable is not set"
    exit 1
fi

if [ -z "${GITHUB_REPOSITORY:-}" ]; then
    echo "Error: GITHUB_REPOSITORY environment variable is not set (format: owner/repo)"
    exit 1
fi

# Get project number
PROJECT_NUMBER=$(gcloud projects describe "${GCP_PROJECT_ID}" --format="value(projectNumber)")
echo "Project Number: ${PROJECT_NUMBER}"

# Create workload identity pool
# echo "Creating workload identity pool..."
# gcloud iam workload-identity-pools create github-pool \
#     --project="${GCP_PROJECT_ID}" \
#     --location="global" \
#     --display-name="GitHub Actions Pool" \
#     --description="Pool for GitHub Actions authentication" \
#     || echo "Pool already exists"

# Create OIDC provider
# echo "Creating OIDC provider..."
# gcloud iam workload-identity-pools providers create-oidc github-provider \
#     --project="${GCP_PROJECT_ID}" \
#     --location="global" \
#     --workload-identity-pool="github-pool" \
#     --display-name="GitHub Provider" \
#    --attribute-mapping="google.subject=assertion.sub,attribute.actor=assertion.actor,attribute.repository=assertion.repository,attribute.repository_owner=assertion.repository_owner" \
#    --issuer-uri="https://token.actions.githubusercontent.com" \
#     || echo "Provider already exists"

# Get the service account email
SERVICE_ACCOUNT_EMAIL="stb-production-gh@${GCP_PROJECT_ID}.iam.gserviceaccount.com"

# Check if service account exists
if ! gcloud iam service-accounts describe "${SERVICE_ACCOUNT_EMAIL}" --project="${GCP_PROJECT_ID}" &>/dev/null; then
    echo "Service account ${SERVICE_ACCOUNT_EMAIL} does not exist. Please run Terraform first."
    exit 1
fi

# Grant workload identity user role
echo "Granting workload identity user role..."
gcloud iam service-accounts add-iam-policy-binding "${SERVICE_ACCOUNT_EMAIL}" \
    --project="${GCP_PROJECT_ID}" \
    --role="roles/iam.workloadIdentityUser" \
    --member="principal://iam.googleapis.com/projects/253621188216/locations/global/workloadIdentityPools/github-actions-pool/subject/SUBJECT_ATTRIBUTE_VALUE"

# Generate the required values for GitHub Secrets
WIF_PROVIDER="projects/${PROJECT_NUMBER}/locations/global/workloadIdentityPools/github-pool/providers/github-provider"

echo ""
echo "✅ Workload Identity Federation setup complete!"
echo ""
echo "Add these secrets to your GitHub repository:"
echo ""
echo "WIF_PROVIDER:"
echo "${WIF_PROVIDER}"
echo ""
echo "WIF_SERVICE_ACCOUNT:"
echo "${SERVICE_ACCOUNT_EMAIL}"
echo ""
echo "To add these secrets:"
echo "1. Go to https://github.com/${GITHUB_REPOSITORY}/settings/secrets/actions"
echo "2. Click 'New repository secret'"
echo "3. Add each secret with the values shown above"
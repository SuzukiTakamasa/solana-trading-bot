#!/bin/bash
set -e

# Function to check if a resource exists
resource_exists() {
  local resource_type=$1
  local resource_id=$2
  
  case $resource_type in
    "workload_identity_pool_provider")
      gcloud iam workload-identity-pools providers describe "$resource_id" \
        --workload-identity-pool="github-actions-pool" \
        --location="global" \
        --project="$TF_VAR_project_id" &>/dev/null
      return $?
      ;;
    "logging_metric")
      gcloud logging metrics describe "$resource_id" \
        --project="$TF_VAR_project_id" &>/dev/null
      return $?
      ;;
  esac
}

# Import existing resources if they exist
echo "Checking for existing resources..."

# Check and import workload identity pool provider
if resource_exists "workload_identity_pool_provider" "github-actions-provider"; then
  echo "Found existing workload identity pool provider. Importing..."
  terraform import google_iam_workload_identity_pool_provider.github \
    "projects/$TF_VAR_project_id/locations/global/workloadIdentityPools/github-actions-pool/providers/github-actions-provider" || true
fi

# Check and import logging metric
METRIC_NAME="${TF_VAR_app_name}_trading_errors"
if resource_exists "logging_metric" "$METRIC_NAME"; then
  echo "Found existing logging metric. Importing..."
  terraform import "google_logging_metric.trading_errors[0]" "$METRIC_NAME" || true
fi

echo "Resource check complete."
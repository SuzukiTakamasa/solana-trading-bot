#!/bin/bash

# Script to create secret values in Google Secret Manager
# This should be run once before applying Terraform

set -e

# Get project ID from terraform variables or environment
PROJECT_ID="${GCP_PROJECT_ID:-$(gcloud config get-value project)}"
ENVIRONMENT="${ENVIRONMENT:-production}"
APP_NAME="${APP_NAME:-stb}"

echo "Using project: $PROJECT_ID"
echo "Environment: $ENVIRONMENT"
echo "App name: $APP_NAME"

# Function to create secret value if it doesn't exist
create_secret_value() {
    local secret_name=$1
    local secret_value=$2
    
    echo "Checking if secret '$secret_name' has a value..."
    
    # Check if the secret has any versions
    if ! gcloud secrets versions list "$secret_name" --project="$PROJECT_ID" --limit=1 2>/dev/null | grep -q "STATE"; then
        echo "Creating value for secret '$secret_name'..."
        echo -n "$secret_value" | gcloud secrets versions add "$secret_name" --data-file=- --project="$PROJECT_ID"
        echo "Secret value created successfully."
    else
        echo "Secret '$secret_name' already has a value."
    fi
}

# Check if secrets need values
echo "Please provide the following secret values (they will not be displayed):"

# Wallet Private Key
echo -n "Enter wallet private key: "
read -s WALLET_PRIVATE_KEY
echo
create_secret_value "${APP_NAME}-wallet-private-key-${ENVIRONMENT}" "$WALLET_PRIVATE_KEY"

# LINE Channel Token  
echo -n "Enter LINE channel token: "
read -s LINE_CHANNEL_TOKEN
echo
create_secret_value "${APP_NAME}-line-channel-token-${ENVIRONMENT}" "$LINE_CHANNEL_TOKEN"

# LINE User ID
echo -n "Enter LINE user ID: "
read -s LINE_USER_ID
echo
create_secret_value "${APP_NAME}-line-user-id-${ENVIRONMENT}" "$LINE_USER_ID"

echo "All secrets have been processed."
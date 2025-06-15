#!/bin/bash

# Deployment script for Solana Trading Bot

set -e

# Configuration
PROJECT_ID=${GCP_PROJECT_ID}
REGION=${GCP_REGION:-us-central1}
SERVICE_NAME="solana-trading-bot"
IMAGE_NAME="gcr.io/${PROJECT_ID}/${SERVICE_NAME}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Starting deployment of Solana Trading Bot${NC}"

# Check if required environment variables are set
if [ -z "$PROJECT_ID" ]; then
    echo -e "${RED}Error: GCP_PROJECT_ID environment variable is not set${NC}"
    exit 1
fi

# Authenticate with GCP
echo -e "${YELLOW}Authenticating with Google Cloud...${NC}"
gcloud auth configure-docker

# Set the project
gcloud config set project ${PROJECT_ID}

# Build the Docker image
echo -e "${YELLOW}Building Docker image...${NC}"
docker build -t ${IMAGE_NAME}:latest .

# Push the image to Google Container Registry
echo -e "${YELLOW}Pushing image to GCR...${NC}"
docker push ${IMAGE_NAME}:latest

# Deploy to Cloud Run
echo -e "${YELLOW}Deploying to Cloud Run...${NC}"
gcloud run deploy ${SERVICE_NAME} \
    --image ${IMAGE_NAME}:latest \
    --region ${REGION} \
    --platform managed \
    --allow-unauthenticated \
    --memory 512Mi \
    --cpu 1 \
    --timeout 300 \
    --max-instances 1 \
    --set-env-vars "SERVER_ONLY=false"

# Get the service URL
SERVICE_URL=$(gcloud run services describe ${SERVICE_NAME} --region ${REGION} --format 'value(status.url)')

echo -e "${GREEN}Deployment complete!${NC}"
echo -e "Service URL: ${SERVICE_URL}"

# Set up Cloud Scheduler for hourly execution
echo -e "${YELLOW}Setting up Cloud Scheduler...${NC}"

# Delete existing job if it exists
gcloud scheduler jobs delete solana-trading-bot-hourly --location ${REGION} --quiet || true

# Create new scheduler job
gcloud scheduler jobs create http solana-trading-bot-hourly \
    --location ${REGION} \
    --schedule "0 * * * *" \
    --uri "${SERVICE_URL}/trigger" \
    --http-method GET \
    --attempt-deadline 540s

echo -e "${GREEN}Cloud Scheduler configured for hourly execution${NC}"

# Create a separate cron job for the initial setup
echo -e "${YELLOW}Creating initial setup job...${NC}"

gcloud scheduler jobs delete solana-trading-bot-initial --location ${REGION} --quiet || true

gcloud scheduler jobs create http solana-trading-bot-initial \
    --location ${REGION} \
    --schedule "*/5 * * * *" \
    --uri "${SERVICE_URL}/health" \
    --http-method GET \
    --attempt-deadline 30s \
    --max-retry-attempts 3

echo -e "${GREEN}All deployments complete!${NC}"
echo -e "${YELLOW}Remember to set the following secrets in GitHub:${NC}"
echo "  - GCP_PROJECT_ID"
echo "  - GCP_SA_KEY (Service Account JSON key)"
echo "  - SOLANA_RPC_URL"
echo "  - WALLET_PRIVATE_KEY"
echo "  - LINE_CHANNEL_TOKEN"
echo "  - LINE_USER_ID"
echo "  - JUPITER_API_URL (optional)"
echo "  - SLIPPAGE_BPS (optional)"
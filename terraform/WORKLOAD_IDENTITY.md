# Workload Identity Federation Setup

This Terraform configuration sets up Workload Identity Federation (WIF) to allow GitHub Actions to authenticate with Google Cloud without using service account keys.

## What's Created

1. **Workload Identity Pool**: A pool named `github-actions-pool` that accepts identities from GitHub Actions
2. **Workload Identity Provider**: An OIDC provider that trusts tokens from GitHub Actions
3. **Service Account**: A dedicated service account for GitHub Actions with necessary permissions
4. **IAM Bindings**: Allows the GitHub repository to impersonate the service account

## Permissions Granted to GitHub Actions

The GitHub Actions service account has the following roles:
- `roles/run.admin` - Manage Cloud Run services
- `roles/iam.serviceAccountUser` - Act as service accounts for deployments
- `roles/storage.admin` - Manage storage buckets and objects
- `roles/artifactregistry.admin` - Push Docker images to Artifact Registry
- `roles/secretmanager.admin` - Manage secrets in Secret Manager
- `roles/cloudscheduler.admin` - Manage Cloud Scheduler jobs
- `roles/logging.viewer` - View logs
- `roles/monitoring.viewer` - View monitoring data

## GitHub Actions Configuration

After running Terraform, you'll get two important outputs:
- `workload_identity_provider`: The full resource name of the WIF provider
- `github_actions_service_account`: The email of the service account

Use these values to set up the following GitHub secrets:
- `WIF_PROVIDER`: Set to the value of `workload_identity_provider` output
- `WIF_SERVICE_ACCOUNT`: Set to the value of `github_actions_service_account` output

## Security Features

- Only the specified GitHub repository can authenticate (configured via `github_repository` variable)
- No service account keys are created or stored
- Authentication tokens are short-lived and automatically rotated
- All actions are auditable in Cloud Logging

## Troubleshooting

If authentication fails:
1. Ensure the repository name in the `github_repository` variable matches exactly (format: `owner/repo`)
2. Verify the GitHub Actions workflow is using the correct WIF provider and service account
3. Check that all required APIs are enabled (iam.googleapis.com, iamcredentials.googleapis.com, sts.googleapis.com)
4. Review Cloud Logging for authentication errors
#!/usr/bin/env bash
#
# LinguaBridge Docker Release Script
#
# Builds and pushes Docker images to GHCR and Docker Hub.
#
# Usage:
#   ./scripts/release.sh [OPTIONS]
#
# Options:
#   --tag TAG          Version tag (default: latest)
#   --ghcr             Push to GitHub Container Registry
#   --dockerhub        Push to Docker Hub
#   --all              Push to both registries (default if no registry specified)
#   --ghcr-owner NAME  Override GHCR_OWNER for this run
#   --dockerhub-owner  Override DOCKERHUB_OWNER for this run
#   --bot-only         Only build/push the bot image
#   --inference-only   Only build/push the inference image
#   --no-cache         Build without Docker cache
#   --dry-run          Show what would be done without executing
#   -h, --help         Show this help message
#
# Environment Variables:
#   GHCR_OWNER       GitHub username OR organization name (required for GHCR)
#   DOCKERHUB_OWNER  Docker Hub username OR organization name (required for Docker Hub)
#
# Examples:
#   # Push to personal account
#   GHCR_OWNER=myusername ./scripts/release.sh --tag v1.0.0 --ghcr
#
#   # Push to GitHub organization
#   GHCR_OWNER=my-org ./scripts/release.sh --tag v1.0.0 --ghcr
#
#   # Push to Docker Hub organization
#   DOCKERHUB_OWNER=my-org ./scripts/release.sh --tag v1.0.0 --dockerhub
#
#   # Use command-line overrides
#   ./scripts/release.sh --tag v1.0.0 --ghcr --ghcr-owner my-org
#
#   # Push to both registries
#   ./scripts/release.sh --tag v1.0.0 --all

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Defaults
TAG="latest"
PUSH_GHCR=false
PUSH_DOCKERHUB=false
BUILD_BOT=true
BUILD_INFERENCE=true
NO_CACHE=""
DRY_RUN=false

# Owner overrides (can be set via CLI)
CLI_GHCR_OWNER=""
CLI_DOCKERHUB_OWNER=""

# Image names
BOT_IMAGE="linguabridge-bot"
INFERENCE_IMAGE="linguabridge-inference"

print_help() {
    sed -n '2,/^$/p' "$0" | sed 's/^# //' | sed 's/^#//'
}

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

run_cmd() {
    if [ "$DRY_RUN" = true ]; then
        echo -e "${YELLOW}[DRY-RUN]${NC} $*"
    else
        "$@"
    fi
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --tag)
            TAG="$2"
            shift 2
            ;;
        --ghcr)
            PUSH_GHCR=true
            shift
            ;;
        --dockerhub)
            PUSH_DOCKERHUB=true
            shift
            ;;
        --all)
            PUSH_GHCR=true
            PUSH_DOCKERHUB=true
            shift
            ;;
        --ghcr-owner)
            CLI_GHCR_OWNER="$2"
            shift 2
            ;;
        --dockerhub-owner)
            CLI_DOCKERHUB_OWNER="$2"
            shift 2
            ;;
        --bot-only)
            BUILD_INFERENCE=false
            shift
            ;;
        --inference-only)
            BUILD_BOT=false
            shift
            ;;
        --no-cache)
            NO_CACHE="--no-cache"
            shift
            ;;
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        -h|--help)
            print_help
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            print_help
            exit 1
            ;;
    esac
done

# If no registry specified, push to both
if [ "$PUSH_GHCR" = false ] && [ "$PUSH_DOCKERHUB" = false ]; then
    PUSH_GHCR=true
    PUSH_DOCKERHUB=true
fi

# Apply CLI overrides to environment variables
if [ -n "$CLI_GHCR_OWNER" ]; then
    GHCR_OWNER="$CLI_GHCR_OWNER"
fi

if [ -n "$CLI_DOCKERHUB_OWNER" ]; then
    DOCKERHUB_OWNER="$CLI_DOCKERHUB_OWNER"
fi

# Validate environment
if [ "$PUSH_GHCR" = true ] && [ -z "${GHCR_OWNER:-}" ]; then
    log_error "GHCR_OWNER environment variable is required for GHCR"
    log_info "Set it with: export GHCR_OWNER=your-username-or-org"
    log_info "Or use: --ghcr-owner your-username-or-org"
    exit 1
fi

if [ "$PUSH_DOCKERHUB" = true ] && [ -z "${DOCKERHUB_OWNER:-}" ]; then
    log_error "DOCKERHUB_OWNER environment variable is required for Docker Hub"
    log_info "Set it with: export DOCKERHUB_OWNER=your-username-or-org"
    log_info "Or use: --dockerhub-owner your-username-or-org"
    exit 1
fi

# Get script directory and project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

log_info "LinguaBridge Docker Release"
log_info "Tag: $TAG"
log_info "Push to GHCR: $PUSH_GHCR"
log_info "Push to Docker Hub: $PUSH_DOCKERHUB"
log_info "Build bot: $BUILD_BOT"
log_info "Build inference: $BUILD_INFERENCE"
echo ""

# Build function
build_image() {
    local name=$1
    local dockerfile=$2

    log_info "Building $name..."
    run_cmd docker build \
        $NO_CACHE \
        -f "$dockerfile" \
        -t "$name:$TAG" \
        -t "$name:latest" \
        .
    log_success "Built $name:$TAG"
}

# Push function
push_image() {
    local local_name=$1
    local remote_name=$2

    log_info "Tagging $local_name as $remote_name..."
    run_cmd docker tag "$local_name:$TAG" "$remote_name:$TAG"
    run_cmd docker tag "$local_name:latest" "$remote_name:latest"

    log_info "Pushing $remote_name:$TAG..."
    run_cmd docker push "$remote_name:$TAG"

    log_info "Pushing $remote_name:latest..."
    run_cmd docker push "$remote_name:latest"

    log_success "Pushed $remote_name"
}

# Login to registries
if [ "$PUSH_GHCR" = true ] && [ "$DRY_RUN" = false ]; then
    log_info "Logging in to GitHub Container Registry..."
    log_info "Owner/Org: $GHCR_OWNER"
    echo "Please enter your GitHub Personal Access Token (with write:packages scope):"
    if [ -t 0 ]; then
        # Interactive mode - use your personal username for auth, even when pushing to org
        read -rp "GitHub username for authentication: " GITHUB_AUTH_USER
        docker login ghcr.io -u "$GITHUB_AUTH_USER"
    else
        log_warn "Non-interactive mode - assuming already logged in to GHCR"
    fi
fi

if [ "$PUSH_DOCKERHUB" = true ] && [ "$DRY_RUN" = false ]; then
    log_info "Logging in to Docker Hub..."
    log_info "Owner/Org: $DOCKERHUB_OWNER"
    if [ -t 0 ]; then
        # Interactive mode - use your personal username for auth, even when pushing to org
        read -rp "Docker Hub username for authentication: " DOCKERHUB_AUTH_USER
        docker login -u "$DOCKERHUB_AUTH_USER"
    else
        log_warn "Non-interactive mode - assuming already logged in to Docker Hub"
    fi
fi

# Build images
if [ "$BUILD_BOT" = true ]; then
    build_image "$BOT_IMAGE" "docker/Dockerfile.rust"
fi

if [ "$BUILD_INFERENCE" = true ]; then
    build_image "$INFERENCE_IMAGE" "docker/Dockerfile.inference"
fi

# Push to GHCR
if [ "$PUSH_GHCR" = true ]; then
    log_info "Pushing to GitHub Container Registry (owner: $GHCR_OWNER)..."

    if [ "$BUILD_BOT" = true ]; then
        push_image "$BOT_IMAGE" "ghcr.io/$GHCR_OWNER/$BOT_IMAGE"
    fi

    if [ "$BUILD_INFERENCE" = true ]; then
        push_image "$INFERENCE_IMAGE" "ghcr.io/$GHCR_OWNER/$INFERENCE_IMAGE"
    fi
fi

# Push to Docker Hub
if [ "$PUSH_DOCKERHUB" = true ]; then
    log_info "Pushing to Docker Hub (owner: $DOCKERHUB_OWNER)..."

    if [ "$BUILD_BOT" = true ]; then
        push_image "$BOT_IMAGE" "$DOCKERHUB_OWNER/$BOT_IMAGE"
    fi

    if [ "$BUILD_INFERENCE" = true ]; then
        push_image "$INFERENCE_IMAGE" "$DOCKERHUB_OWNER/$INFERENCE_IMAGE"
    fi
fi

echo ""
log_success "Release complete!"
echo ""

# Print image references
log_info "Image references for deploy.yaml:"
echo ""
if [ "$BUILD_BOT" = true ]; then
    if [ "$PUSH_GHCR" = true ]; then
        echo "  GHCR Bot:           ghcr.io/$GHCR_OWNER/$BOT_IMAGE:$TAG"
    fi
    if [ "$PUSH_DOCKERHUB" = true ]; then
        echo "  Docker Hub Bot:     $DOCKERHUB_OWNER/$BOT_IMAGE:$TAG"
    fi
fi
if [ "$BUILD_INFERENCE" = true ]; then
    if [ "$PUSH_GHCR" = true ]; then
        echo "  GHCR Inference:     ghcr.io/$GHCR_OWNER/$INFERENCE_IMAGE:$TAG"
    fi
    if [ "$PUSH_DOCKERHUB" = true ]; then
        echo "  Docker Hub Inference: $DOCKERHUB_OWNER/$INFERENCE_IMAGE:$TAG"
    fi
fi
echo ""
log_info "Update your deploy.yaml with these image references before deploying to Akash."

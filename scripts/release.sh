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
#   --tag TAG        Version tag (default: latest)
#   --ghcr           Push to GitHub Container Registry
#   --dockerhub      Push to Docker Hub
#   --all            Push to both registries (default if no registry specified)
#   --bot-only       Only build/push the bot image
#   --inference-only Only build/push the inference image
#   --no-cache       Build without Docker cache
#   --dry-run        Show what would be done without executing
#   -h, --help       Show this help message
#
# Environment Variables:
#   GITHUB_USER      GitHub username (required for GHCR)
#   DOCKERHUB_USER   Docker Hub username (required for Docker Hub)
#
# Examples:
#   ./scripts/release.sh --tag v1.0.0 --all
#   ./scripts/release.sh --tag latest --ghcr
#   ./scripts/release.sh --inference-only --dockerhub --tag v1.0.0

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

# Validate environment
if [ "$PUSH_GHCR" = true ] && [ -z "${GITHUB_USER:-}" ]; then
    log_error "GITHUB_USER environment variable is required for GHCR"
    log_info "Set it with: export GITHUB_USER=yourusername"
    exit 1
fi

if [ "$PUSH_DOCKERHUB" = true ] && [ -z "${DOCKERHUB_USER:-}" ]; then
    log_error "DOCKERHUB_USER environment variable is required for Docker Hub"
    log_info "Set it with: export DOCKERHUB_USER=yourusername"
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
    echo "Please enter your GitHub Personal Access Token (with write:packages scope):"
    if [ -t 0 ]; then
        # Interactive mode
        docker login ghcr.io -u "$GITHUB_USER"
    else
        log_warn "Non-interactive mode - assuming already logged in to GHCR"
    fi
fi

if [ "$PUSH_DOCKERHUB" = true ] && [ "$DRY_RUN" = false ]; then
    log_info "Logging in to Docker Hub..."
    if [ -t 0 ]; then
        # Interactive mode
        docker login -u "$DOCKERHUB_USER"
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
    log_info "Pushing to GitHub Container Registry..."

    if [ "$BUILD_BOT" = true ]; then
        push_image "$BOT_IMAGE" "ghcr.io/$GITHUB_USER/$BOT_IMAGE"
    fi

    if [ "$BUILD_INFERENCE" = true ]; then
        push_image "$INFERENCE_IMAGE" "ghcr.io/$GITHUB_USER/$INFERENCE_IMAGE"
    fi
fi

# Push to Docker Hub
if [ "$PUSH_DOCKERHUB" = true ]; then
    log_info "Pushing to Docker Hub..."

    if [ "$BUILD_BOT" = true ]; then
        push_image "$BOT_IMAGE" "$DOCKERHUB_USER/$BOT_IMAGE"
    fi

    if [ "$BUILD_INFERENCE" = true ]; then
        push_image "$INFERENCE_IMAGE" "$DOCKERHUB_USER/$INFERENCE_IMAGE"
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
        echo "  GHCR Bot:       ghcr.io/$GITHUB_USER/$BOT_IMAGE:$TAG"
    fi
    if [ "$PUSH_DOCKERHUB" = true ]; then
        echo "  DockerHub Bot:  $DOCKERHUB_USER/$BOT_IMAGE:$TAG"
    fi
fi
if [ "$BUILD_INFERENCE" = true ]; then
    if [ "$PUSH_GHCR" = true ]; then
        echo "  GHCR Inference: ghcr.io/$GITHUB_USER/$INFERENCE_IMAGE:$TAG"
    fi
    if [ "$PUSH_DOCKERHUB" = true ]; then
        echo "  DockerHub Inference: $DOCKERHUB_USER/$INFERENCE_IMAGE:$TAG"
    fi
fi
echo ""
log_info "Update your deploy.yaml with these image references before deploying to Akash."

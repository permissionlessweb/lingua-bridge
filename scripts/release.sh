#!/usr/bin/env bash
#
# LinguaBridge Docker Release Script
#
# Builds and pushes Docker images to GHCR and Docker Hub with multi-architecture support.
#
# Usage:
#   ./scripts/release.sh [OPTIONS]
#
# Options:
#   --tag TAG            Version tag (default: latest)
#   --ghcr               Push to GitHub Container Registry
#   --dockerhub          Push to Docker Hub
#   --all                Push to both registries (default if no registry specified)
#   --ghcr-owner NAME    Override GHCR_OWNER for this run
#   --dockerhub-owner    Override DOCKERHUB_OWNER for this run
#   --bot-only           Only build/push the bot image
#   --inference-only     Only build/push the inference image
#   --no-cache           Build without Docker cache
#   --dry-run            Show what would be done without executing
#   -h, --help           Show this help message
#
# Architecture Options:
#   --platform PLATFORMS Comma-separated platforms (default: linux/amd64)
#                        Examples: linux/amd64, linux/arm64, linux/amd64,linux/arm64
#   --native             Build only for native architecture (auto-detected)
#   --multi              Build for both amd64 and arm64
#
# Environment Variables:
#   GHCR_OWNER       GitHub username OR organization name (required for GHCR)
#   DOCKERHUB_OWNER  Docker Hub username OR organization name (required for Docker Hub)
#
# Examples:
#   # Build for amd64 only (default)
#   GHCR_OWNER=myusername ./scripts/release.sh --tag v1.0.0 --ghcr
#
#   # Build for native architecture (e.g., arm64 on Apple Silicon)
#   ./scripts/release.sh --tag v1.0.0 --ghcr --native
#
#   # Build multi-arch images (amd64 + arm64)
#   ./scripts/release.sh --tag v1.0.0 --ghcr --multi
#
#   # Specify exact platforms
#   ./scripts/release.sh --tag v1.0.0 --platform linux/amd64,linux/arm64 --ghcr
#
#   # Build for Akash Network (typically amd64)
#   ./scripts/release.sh --tag v1.0.0 --platform linux/amd64 --all

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Defaults
TAG="latest"
PUSH_GHCR=false
PUSH_DOCKERHUB=false
BUILD_BOT=true
BUILD_INFERENCE=true
NO_CACHE=""
DRY_RUN=false
PLATFORMS="linux/amd64"
USE_NATIVE=false
USE_MULTI=false

# Owner overrides (can be set via CLI)
CLI_GHCR_OWNER=""
CLI_DOCKERHUB_OWNER=""

# Image names
BOT_IMAGE="linguabridge-bot"
INFERENCE_IMAGE="linguabridge-inference"

# Buildx builder name
BUILDER_NAME="linguabridge-builder"

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

log_arch() {
    echo -e "${CYAN}[ARCH]${NC} $1"
}

run_cmd() {
    if [ "$DRY_RUN" = true ]; then
        echo -e "${YELLOW}[DRY-RUN]${NC} $*"
    else
        "$@"
    fi
}

# Detect native architecture
detect_native_arch() {
    local arch
    arch=$(uname -m)
    case "$arch" in
        x86_64|amd64)
            echo "linux/amd64"
            ;;
        aarch64|arm64)
            echo "linux/arm64"
            ;;
        armv7l)
            echo "linux/arm/v7"
            ;;
        *)
            log_warn "Unknown architecture: $arch, defaulting to linux/amd64"
            echo "linux/amd64"
            ;;
    esac
}

# Setup buildx builder for multi-platform builds
setup_buildx() {
    log_info "Setting up Docker buildx for multi-platform builds..."

    # Check if builder exists
    if docker buildx inspect "$BUILDER_NAME" &>/dev/null; then
        log_info "Using existing buildx builder: $BUILDER_NAME"
    else
        log_info "Creating new buildx builder: $BUILDER_NAME"
        run_cmd docker buildx create --name "$BUILDER_NAME" --driver docker-container --bootstrap
    fi

    run_cmd docker buildx use "$BUILDER_NAME"
    log_success "Buildx builder ready"
}

# Check if we need multi-platform support
needs_multiplatform() {
    [[ "$PLATFORMS" == *","* ]]
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
        --platform)
            PLATFORMS="$2"
            shift 2
            ;;
        --native)
            USE_NATIVE=true
            shift
            ;;
        --multi)
            USE_MULTI=true
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

# Handle architecture options
if [ "$USE_NATIVE" = true ]; then
    PLATFORMS=$(detect_native_arch)
    log_arch "Using native architecture: $PLATFORMS"
elif [ "$USE_MULTI" = true ]; then
    PLATFORMS="linux/amd64,linux/arm64"
    log_arch "Using multi-architecture: $PLATFORMS"
fi

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

# Warn about CUDA arm64 limitation
if [ "$BUILD_INFERENCE" = true ] && [[ "$PLATFORMS" == *"arm64"* ]]; then
    log_warn "NVIDIA CUDA images have limited arm64 support."
    log_warn "The inference image may only build successfully for linux/amd64."
    log_warn "Consider using --bot-only for arm64-only builds, or accept amd64-only inference."
    echo ""
fi

# Get script directory and project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

echo ""
log_info "========================================="
log_info "  LinguaBridge Docker Release"
log_info "========================================="
echo ""
log_info "Tag: $TAG"
log_arch "Platforms: $PLATFORMS"
log_info "Push to GHCR: $PUSH_GHCR"
log_info "Push to Docker Hub: $PUSH_DOCKERHUB"
log_info "Build bot: $BUILD_BOT"
log_info "Build inference: $BUILD_INFERENCE"
echo ""

# Setup buildx for multi-platform or cross-platform builds
NATIVE_ARCH=$(detect_native_arch)
if needs_multiplatform || [ "$PLATFORMS" != "$NATIVE_ARCH" ]; then
    setup_buildx
fi

# Build function for multi-platform with buildx
build_and_push_image() {
    local name=$1
    local dockerfile=$2
    local platforms=$3
    shift 3
    local registries=("$@")

    log_info "Building $name for $platforms..."

    # Build tag arguments
    local tag_args=""
    for registry in "${registries[@]}"; do
        tag_args="$tag_args -t $registry/$name:$TAG -t $registry/$name:latest"
    done

    # For multi-platform builds, we must push directly (can't load to local daemon)
    if needs_multiplatform; then
        log_info "Multi-platform build: pushing directly to registries..."
        run_cmd docker buildx build \
            --platform "$platforms" \
            $NO_CACHE \
            -f "$dockerfile" \
            $tag_args \
            --push \
            .
    else
        # Single platform: build and load locally, then push
        local local_tag="$name:$TAG"
        run_cmd docker buildx build \
            --platform "$platforms" \
            $NO_CACHE \
            -f "$dockerfile" \
            -t "$local_tag" \
            -t "$name:latest" \
            --load \
            .

        # Push to each registry
        for registry in "${registries[@]}"; do
            log_info "Pushing to $registry..."
            run_cmd docker tag "$local_tag" "$registry/$name:$TAG"
            run_cmd docker tag "$name:latest" "$registry/$name:latest"
            run_cmd docker push "$registry/$name:$TAG"
            run_cmd docker push "$registry/$name:latest"
        done
    fi

    log_success "Built and pushed $name"
}

# Login to registries
if [ "$PUSH_GHCR" = true ] && [ "$DRY_RUN" = false ]; then
    log_info "Logging in to GitHub Container Registry..."
    log_info "Owner/Org: $GHCR_OWNER"
    echo "Please enter your GitHub Personal Access Token (with write:packages scope):"
    if [ -t 0 ]; then
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
        read -rp "Docker Hub username for authentication: " DOCKERHUB_AUTH_USER
        docker login -u "$DOCKERHUB_AUTH_USER"
    else
        log_warn "Non-interactive mode - assuming already logged in to Docker Hub"
    fi
fi

# Build registry list
REGISTRIES=()
if [ "$PUSH_GHCR" = true ]; then
    REGISTRIES+=("ghcr.io/$GHCR_OWNER")
fi
if [ "$PUSH_DOCKERHUB" = true ]; then
    REGISTRIES+=("$DOCKERHUB_OWNER")
fi

# Build images
if [ "$BUILD_BOT" = true ]; then
    build_and_push_image "$BOT_IMAGE" "docker/Dockerfile.rust" "$PLATFORMS" "${REGISTRIES[@]}"
fi

if [ "$BUILD_INFERENCE" = true ]; then
    # For inference, limit to amd64 if multi-arch requested (CUDA limitation)
    INFERENCE_PLATFORMS="$PLATFORMS"
    if [[ "$PLATFORMS" == *"arm64"* ]] && [[ "$PLATFORMS" == *"amd64"* ]]; then
        log_warn "Limiting inference build to linux/amd64 (CUDA limitation)"
        INFERENCE_PLATFORMS="linux/amd64"
    fi
    build_and_push_image "$INFERENCE_IMAGE" "docker/Dockerfile.inference" "$INFERENCE_PLATFORMS" "${REGISTRIES[@]}"
fi

echo ""
log_success "========================================="
log_success "  Release complete!"
log_success "========================================="
echo ""

# Print image references
log_info "Image references for deploy.yaml:"
echo ""
if [ "$BUILD_BOT" = true ]; then
    if [ "$PUSH_GHCR" = true ]; then
        echo "  GHCR Bot:             ghcr.io/$GHCR_OWNER/$BOT_IMAGE:$TAG"
    fi
    if [ "$PUSH_DOCKERHUB" = true ]; then
        echo "  Docker Hub Bot:       $DOCKERHUB_OWNER/$BOT_IMAGE:$TAG"
    fi
    echo "  Platforms:            $PLATFORMS"
fi
echo ""
if [ "$BUILD_INFERENCE" = true ]; then
    if [ "$PUSH_GHCR" = true ]; then
        echo "  GHCR Inference:       ghcr.io/$GHCR_OWNER/$INFERENCE_IMAGE:$TAG"
    fi
    if [ "$PUSH_DOCKERHUB" = true ]; then
        echo "  Docker Hub Inference: $DOCKERHUB_OWNER/$INFERENCE_IMAGE:$TAG"
    fi
    if [[ "$PLATFORMS" == *"arm64"* ]]; then
        echo "  Platforms:            linux/amd64 (CUDA requires x86_64)"
    else
        echo "  Platforms:            $PLATFORMS"
    fi
fi
echo ""
log_info "Update your deploy.yaml with these image references before deploying to Akash."
echo ""

# Architecture summary
log_arch "Architecture Summary:"
echo "  Native:  $NATIVE_ARCH"
echo "  Built:   $PLATFORMS"
if [[ "$PLATFORMS" != "$NATIVE_ARCH" ]]; then
    echo "  Note:    Cross-compilation was used (buildx)"
fi

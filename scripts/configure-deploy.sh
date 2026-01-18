#!/usr/bin/env bash
#
# LinguaBridge Akash SDL Configuration Script
#
# Interactive wizard that configures deploy.yaml for Akash Network deployment.
# Uses deploy.yaml as a template and generates a configured deployment file.
#
# Usage:
#   ./scripts/configure-deploy.sh [OPTIONS]
#
# Options:
#   -o, --output FILE    Output file (default: deploy-configured.yaml)
#   -t, --template FILE  Template file (default: deploy.yaml)
#   --non-interactive    Use environment variables instead of prompts
#   -h, --help           Show this help message
#
# Environment Variables (for non-interactive mode):
#   GHCR_USERNAME        GitHub username for registry auth
#   GHCR_PAT             GitHub Personal Access Token (read:packages)
#   GHCR_ORG             GitHub org/user for image URLs
#   IMAGE_TAG            Docker image tag (default: latest)
#   ADMIN_PUBLIC_KEY     Admin public key (or path to admin.pub)
#
# Examples:
#   # Interactive mode (recommended)
#   ./scripts/configure-deploy.sh
#
#   # Non-interactive with environment variables
#   GHCR_USERNAME=myuser GHCR_PAT=ghp_xxx GHCR_ORG=myorg \
#     ./scripts/configure-deploy.sh --non-interactive

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

# Defaults
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
TEMPLATE_FILE="$PROJECT_ROOT/deploy.yaml"
OUTPUT_FILE="$PROJECT_ROOT/deploy-configured.yaml"
NON_INTERACTIVE=false

# Configuration values (will be populated)
CONFIG_GHCR_USERNAME=""
CONFIG_GHCR_PAT=""
CONFIG_GHCR_ORG=""
CONFIG_IMAGE_TAG="latest"
CONFIG_ADMIN_PUBKEY=""
CONFIG_PUBLIC_URL=""

print_help() {
    sed -n '2,/^$/p' "$0" | sed 's/^# //' | sed 's/^#//'
}

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[OK]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_header() {
    echo ""
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${CYAN}  $1${NC}"
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
}

print_step() {
    echo ""
    echo -e "${BOLD}$1${NC}"
    echo -e "${BLUE}─────────────────────────────────────────${NC}"
}

# Prompt for input with default value
prompt() {
    local prompt_text=$1
    local default_value=${2:-}
    local var_name=$3
    local is_secret=${4:-false}

    if [ -n "$default_value" ]; then
        echo -en "${prompt_text} [${CYAN}${default_value}${NC}]: "
    else
        echo -en "${prompt_text}: "
    fi

    if [ "$is_secret" = true ]; then
        read -rs value
        echo ""
    else
        read -r value
    fi

    if [ -z "$value" ] && [ -n "$default_value" ]; then
        value="$default_value"
    fi

    eval "$var_name=\"$value\""
}

# Prompt for yes/no
prompt_yn() {
    local prompt_text=$1
    local default=${2:-n}

    if [ "$default" = "y" ]; then
        echo -en "${prompt_text} [${CYAN}Y/n${NC}]: "
    else
        echo -en "${prompt_text} [${CYAN}y/N${NC}]: "
    fi

    read -r answer
    answer=${answer:-$default}

    [[ "$answer" =~ ^[Yy] ]]
}

# Check if admin keys exist
check_admin_keys() {
    if [ -f "$PROJECT_ROOT/admin.pub" ]; then
        return 0
    elif [ -f "$PROJECT_ROOT/admin.key" ]; then
        # Key exists but no pub file - regenerate pub from key
        log_warn "admin.key exists but admin.pub not found"
        return 1
    else
        return 1
    fi
}

# Generate admin keys
generate_admin_keys() {
    log_info "Generating admin keypair..."

    if command -v cargo &>/dev/null; then
        (cd "$PROJECT_ROOT" && cargo run -p admin-cli --release -- keygen)
        log_success "Admin keys generated: admin.key and admin.pub"
        return 0
    else
        log_error "Cargo not found. Please install Rust or generate keys manually."
        log_info "Run: cargo run -p admin-cli -- keygen"
        return 1
    fi
}

# Read admin public key
read_admin_pubkey() {
    local pubkey_file="$PROJECT_ROOT/admin.pub"

    if [ -f "$pubkey_file" ]; then
        cat "$pubkey_file"
    else
        echo ""
    fi
}

# Validate GitHub PAT format
validate_ghcr_pat() {
    local pat=$1

    # GitHub PATs start with ghp_, github_pat_, or are classic tokens
    if [[ "$pat" =~ ^ghp_ ]] || [[ "$pat" =~ ^github_pat_ ]] || [ ${#pat} -ge 40 ]; then
        return 0
    else
        return 1
    fi
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -o|--output)
            OUTPUT_FILE="$2"
            shift 2
            ;;
        -t|--template)
            TEMPLATE_FILE="$2"
            shift 2
            ;;
        --non-interactive)
            NON_INTERACTIVE=true
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

# Check template exists
if [ ! -f "$TEMPLATE_FILE" ]; then
    log_error "Template file not found: $TEMPLATE_FILE"
    exit 1
fi

# Main configuration flow
print_header "LinguaBridge Akash Deployment Configuration"

echo "This wizard will configure your Akash SDL deployment file."
echo "It will:"
echo "  1. Set up GHCR (GitHub Container Registry) credentials"
echo "  2. Configure Docker image references"
echo "  3. Set up admin provisioning keys"
echo "  4. Generate a ready-to-deploy SDL file"
echo ""

if [ "$NON_INTERACTIVE" = true ]; then
    # Non-interactive mode: use environment variables
    log_info "Running in non-interactive mode..."

    CONFIG_GHCR_USERNAME="${GHCR_USERNAME:-}"
    CONFIG_GHCR_PAT="${GHCR_PAT:-}"
    CONFIG_GHCR_ORG="${GHCR_ORG:-$CONFIG_GHCR_USERNAME}"
    CONFIG_IMAGE_TAG="${IMAGE_TAG:-latest}"

    # Handle admin public key
    if [ -n "${ADMIN_PUBLIC_KEY:-}" ]; then
        if [ -f "$ADMIN_PUBLIC_KEY" ]; then
            CONFIG_ADMIN_PUBKEY=$(cat "$ADMIN_PUBLIC_KEY")
        else
            CONFIG_ADMIN_PUBKEY="$ADMIN_PUBLIC_KEY"
        fi
    elif [ -f "$PROJECT_ROOT/admin.pub" ]; then
        CONFIG_ADMIN_PUBKEY=$(cat "$PROJECT_ROOT/admin.pub")
    fi

    # Validate required values
    if [ -z "$CONFIG_GHCR_USERNAME" ]; then
        log_error "GHCR_USERNAME is required in non-interactive mode"
        exit 1
    fi
    if [ -z "$CONFIG_GHCR_PAT" ]; then
        log_error "GHCR_PAT is required in non-interactive mode"
        exit 1
    fi
    if [ -z "$CONFIG_ADMIN_PUBKEY" ]; then
        log_error "ADMIN_PUBLIC_KEY is required (or admin.pub must exist)"
        exit 1
    fi
else
    # Interactive mode

    # Step 1: Admin Keys
    print_step "Step 1: Admin Provisioning Keys"

    echo "Admin keys are used for secure credential provisioning."
    echo "The public key is embedded in the SDL; the private key stays with you."
    echo ""

    if check_admin_keys; then
        CONFIG_ADMIN_PUBKEY=$(read_admin_pubkey)
        log_success "Found existing admin.pub"
        echo "  Public key: ${CONFIG_ADMIN_PUBKEY:0:20}..."
        echo ""

        if prompt_yn "Use this existing key?"; then
            : # Keep the key
        else
            if prompt_yn "Generate new admin keys? (overwrites existing)"; then
                generate_admin_keys
                CONFIG_ADMIN_PUBKEY=$(read_admin_pubkey)
            fi
        fi
    else
        log_warn "No admin keys found"

        if prompt_yn "Generate admin keys now?" "y"; then
            if generate_admin_keys; then
                CONFIG_ADMIN_PUBKEY=$(read_admin_pubkey)
            else
                echo ""
                prompt "Enter admin public key manually" "" CONFIG_ADMIN_PUBKEY
            fi
        else
            prompt "Enter admin public key" "" CONFIG_ADMIN_PUBKEY
        fi
    fi

    if [ -z "$CONFIG_ADMIN_PUBKEY" ]; then
        log_error "Admin public key is required"
        exit 1
    fi

    # Step 2: GHCR Credentials
    print_step "Step 2: GitHub Container Registry Credentials"

    echo "These credentials allow Akash providers to pull your private images."
    echo "You need a GitHub Personal Access Token with 'read:packages' scope."
    echo ""
    echo "Create one at: https://github.com/settings/tokens/new"
    echo "  - Select scope: read:packages"
    echo ""

    prompt "GitHub username (for auth)" "${GHCR_USERNAME:-}" CONFIG_GHCR_USERNAME

    echo ""
    echo "Enter your GitHub PAT (input hidden):"
    prompt "GitHub PAT" "" CONFIG_GHCR_PAT true

    if ! validate_ghcr_pat "$CONFIG_GHCR_PAT"; then
        log_warn "PAT format looks unusual. Make sure it's a valid GitHub token."
    fi

    # Step 3: Image Configuration
    print_step "Step 3: Docker Image Configuration"

    echo "Configure the Docker image references for your deployment."
    echo ""

    # Default org to username if not set
    default_org="${GHCR_ORG:-${CONFIG_GHCR_USERNAME}}"
    prompt "GitHub org/user for images" "$default_org" CONFIG_GHCR_ORG

    prompt "Image tag" "latest" CONFIG_IMAGE_TAG

    echo ""
    echo "Image URLs will be:"
    echo "  Bot:       ghcr.io/${CONFIG_GHCR_ORG}/linguabridge-bot:${CONFIG_IMAGE_TAG}"
    echo "  Inference: ghcr.io/${CONFIG_GHCR_ORG}/linguabridge-inference:${CONFIG_IMAGE_TAG}"

    # Step 4: Optional Settings
    print_step "Step 4: Optional Settings"

    echo "These can be updated after deployment if needed."
    echo ""

    prompt "Public URL (leave blank to configure later)" "" CONFIG_PUBLIC_URL
fi

# Generate the configured SDL
print_step "Generating Configured SDL"

log_info "Reading template: $TEMPLATE_FILE"
log_info "Writing output: $OUTPUT_FILE"

# Read template and perform substitutions
SDL_CONTENT=$(cat "$TEMPLATE_FILE")

# Substitute values
SDL_CONTENT="${SDL_CONTENT//<GHCR_USERNAME>/$CONFIG_GHCR_USERNAME}"
SDL_CONTENT="${SDL_CONTENT//<GHCR_PAT>/$CONFIG_GHCR_PAT}"
SDL_CONTENT="${SDL_CONTENT//<YOUR_ADMIN_PUBLIC_KEY>/$CONFIG_ADMIN_PUBKEY}"

# Update image URLs
SDL_CONTENT="${SDL_CONTENT//ghcr.io\/permissionlessweb\/linguabridge-bot:latest/ghcr.io\/${CONFIG_GHCR_ORG}\/linguabridge-bot:${CONFIG_IMAGE_TAG}}"
SDL_CONTENT="${SDL_CONTENT//ghcr.io\/permissionlessweb\/linguabridge-inference:latest/ghcr.io\/${CONFIG_GHCR_ORG}\/linguabridge-inference:${CONFIG_IMAGE_TAG}}"

# Update public URL if provided
if [ -n "$CONFIG_PUBLIC_URL" ]; then
    SDL_CONTENT="${SDL_CONTENT//https:\/\/your-deployment.akash.network/$CONFIG_PUBLIC_URL}"
fi

# Write output file
echo "$SDL_CONTENT" > "$OUTPUT_FILE"

log_success "Configuration complete!"

# Summary
print_header "Deployment Summary"

echo -e "${BOLD}Generated File:${NC} $OUTPUT_FILE"
echo ""
echo -e "${BOLD}Configuration:${NC}"
echo "  GHCR Username:    $CONFIG_GHCR_USERNAME"
echo "  GHCR Org/User:    $CONFIG_GHCR_ORG"
echo "  Image Tag:        $CONFIG_IMAGE_TAG"
echo "  Admin Public Key: ${CONFIG_ADMIN_PUBKEY:0:20}..."
if [ -n "$CONFIG_PUBLIC_URL" ]; then
    echo "  Public URL:       $CONFIG_PUBLIC_URL"
fi
echo ""
echo -e "${BOLD}Images:${NC}"
echo "  ghcr.io/${CONFIG_GHCR_ORG}/linguabridge-bot:${CONFIG_IMAGE_TAG}"
echo "  ghcr.io/${CONFIG_GHCR_ORG}/linguabridge-inference:${CONFIG_IMAGE_TAG}"
echo ""

print_header "Next Steps"

echo "1. ${BOLD}Build and push Docker images${NC} (if not already done):"
echo ""
echo "   GHCR_OWNER=${CONFIG_GHCR_ORG} ./scripts/release.sh \\"
echo "     --tag ${CONFIG_IMAGE_TAG} --platform linux/amd64 --ghcr"
echo ""
echo "2. ${BOLD}Deploy to Akash${NC}:"
echo ""
echo "   # Using Akash Console (recommended):"
echo "   # Upload ${OUTPUT_FILE} at https://console.akash.network"
echo ""
echo "   # Or using Akash CLI:"
echo "   akash tx deployment create ${OUTPUT_FILE} --from your-wallet"
echo ""
echo "3. ${BOLD}After deployment, provision the bot${NC}:"
echo ""
echo "   cargo run -p admin-cli --release -- provision \\"
echo "     --bot-url https://<your-akash-uri>:9999 \\"
echo "     --discord-token \"YOUR_DISCORD_BOT_TOKEN\" \\"
echo "     --admin-key admin.key"
echo ""

# Security reminder
echo -e "${YELLOW}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${YELLOW}  Security Reminder${NC}"
echo -e "${YELLOW}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo "  - Keep admin.key secure - it controls bot provisioning"
echo "  - The generated SDL contains your GHCR PAT - don't commit it"
echo "  - Add ${OUTPUT_FILE} to .gitignore"
echo ""

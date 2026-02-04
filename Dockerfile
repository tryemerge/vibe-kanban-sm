# syntax=docker/dockerfile:1.4
# Build stage
FROM node:24-alpine AS builder

# Install build dependencies (rarely changes - cached)
RUN apk add --no-cache \
    curl \
    build-base \
    perl \
    llvm-dev \
    clang-dev

# Allow linking libclang on musl
ENV RUSTFLAGS="-C target-feature=-crt-static"

# Install Rust (rarely changes - cached)
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Set working directory
WORKDIR /app

# ============================================
# NODE DEPENDENCY CACHING
# Copy package files first for layer caching
# ============================================
COPY package*.json pnpm-lock.yaml pnpm-workspace.yaml ./
COPY frontend/package*.json ./frontend/
COPY npx-cli/package*.json ./npx-cli/

# Install pnpm and dependencies (cached unless package files change)
RUN npm install -g pnpm && pnpm install

# ============================================
# COPY ALL SOURCE CODE
# ============================================
COPY . .

# Build args for frontend
ARG POSTHOG_API_KEY
ARG POSTHOG_API_ENDPOINT
ENV VITE_PUBLIC_POSTHOG_KEY=$POSTHOG_API_KEY
ENV VITE_PUBLIC_POSTHOG_HOST=$POSTHOG_API_ENDPOINT

# ============================================
# BUILD WITH BUILDKIT CACHE MOUNTS
# Cargo target and registry are cached between builds
# ============================================

# Generate TypeScript types (uses cached cargo registry + target)
RUN --mount=type=cache,target=/root/.cargo/registry \
    --mount=type=cache,target=/root/.cargo/git \
    --mount=type=cache,target=/app/target \
    SQLX_OFFLINE=true cargo build --release --bin generate_types && \
    ./target/release/generate_types

# Build frontend
RUN cd frontend && NODE_OPTIONS="--max-old-space-size=4096" pnpm run build
RUN echo "=== Checking frontend/dist ===" && ls -la frontend/dist/ && test -f frontend/dist/index.html

# Build Rust server (uses cached deps from generate_types build)
RUN --mount=type=cache,target=/root/.cargo/registry \
    --mount=type=cache,target=/root/.cargo/git \
    --mount=type=cache,target=/app/target \
    SQLX_OFFLINE=true cargo build --release --bin server && \
    cp /app/target/release/server /app/server-binary

# ============================================
# Runtime stage
# ============================================
FROM alpine:latest AS runtime

# Install runtime dependencies
# - Node.js: Required for running coding agents (Claude Code, Codex, etc.) via npx
# - git: Required for worktree operations (workspaces)
# - openssh: For git operations over SSH
# - github-cli: For GitHub integration (cloning, PRs, etc.)
RUN apk add --no-cache \
    ca-certificates \
    tini \
    libgcc \
    wget \
    nodejs \
    npm \
    git \
    openssh-client \
    github-cli

# Install pnpm globally for faster package management
RUN npm install -g pnpm

# Create app user for security with a home directory
RUN addgroup -g 1001 -S appgroup && \
    adduser -u 1001 -S appuser -G appgroup -h /home/appuser

# Copy binary from builder (from the non-cached location)
COPY --from=builder /app/server-binary /usr/local/bin/server

# Copy entrypoint script
COPY docker-entrypoint.sh /usr/local/bin/docker-entrypoint.sh
RUN chmod +x /usr/local/bin/docker-entrypoint.sh

# Create repos directory and set permissions
RUN mkdir -p /repos /repos/.claude-config && \
    chown -R appuser:appgroup /repos /home/appuser

# Setup profile script for root SSH sessions (auto-symlink credentials)
RUN echo '[ -d /repos/.claude-config ] && ln -sf /repos/.claude-config /root/.claude 2>/dev/null; [ -f /repos/.claude-config/.claude.json ] && ln -sf /repos/.claude-config/.claude.json /root/.claude.json 2>/dev/null' > /etc/profile.d/claude-setup.sh

# Switch to non-root user
USER appuser

# Set runtime environment
ENV HOST=0.0.0.0
ENV PORT=3000
EXPOSE 3000

# Set working directory
WORKDIR /repos

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=30s --retries=3 \
    CMD wget --quiet --tries=1 --spider "http://${HOST:-localhost}:${PORT:-3000}/api/health" || exit 1

# Run the application with entrypoint that sets up credentials
ENTRYPOINT ["/sbin/tini", "--", "/usr/local/bin/docker-entrypoint.sh"]
CMD ["server"]

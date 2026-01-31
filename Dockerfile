# Build stage
FROM node:24-alpine AS builder

# Install build dependencies
RUN apk add --no-cache \
    curl \
    build-base \
    perl \
    llvm-dev \
    clang-dev

# Allow linking libclang on musl
ENV RUSTFLAGS="-C target-feature=-crt-static"

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

ARG POSTHOG_API_KEY
ARG POSTHOG_API_ENDPOINT

ENV VITE_PUBLIC_POSTHOG_KEY=$POSTHOG_API_KEY
ENV VITE_PUBLIC_POSTHOG_HOST=$POSTHOG_API_ENDPOINT

# Set working directory
WORKDIR /app

# Copy package files for dependency caching
COPY package*.json pnpm-lock.yaml pnpm-workspace.yaml ./
COPY frontend/package*.json ./frontend/
COPY npx-cli/package*.json ./npx-cli/

# Install pnpm and dependencies
RUN npm install -g pnpm && pnpm install

# Copy source code
COPY . .

# Build application
RUN npm run generate-types
# Increase Node heap size for frontend build (Vite needs more memory)
RUN cd frontend && NODE_OPTIONS="--max-old-space-size=4096" pnpm run build
# Verify frontend was built (fail if index.html missing)
RUN echo "=== Checking frontend/dist ===" && ls -la frontend/dist/ && test -f frontend/dist/index.html && echo "=== index.html found ==="
RUN cargo build --release --bin server

# Runtime stage
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

# Copy binary from builder
COPY --from=builder /app/target/release/server /usr/local/bin/server

# Create repos directory and set permissions
# Also create .claude symlink to persist credentials on the volume
RUN mkdir -p /repos /repos/.claude-config && \
    chown -R appuser:appgroup /repos /home/appuser && \
    ln -sf /repos/.claude-config /home/appuser/.claude

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

# Run the application
ENTRYPOINT ["/sbin/tini", "--"]
CMD ["server"]

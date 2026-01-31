# ADR 2026-01-23-005: Cloud Deployment with Fly.io

## Status
Proposed

## Context
Vibe Kanban needs to be accessible from anywhere, not just the local development machine. This enables:

1. **Remote work**: Access the kanban board and run agents from any location
2. **Always-on agents**: Workflows can continue running without keeping a local machine on
3. **Collaboration**: Multiple team members can access the same deployment

### Alternatives Considered

#### Self-Hosted Mac Mini
A popular approach in the AI agent community is running dedicated Mac minis for agent workloads. Benefits:
- **Local LLM inference**: M-series unified memory is excellent for running local models (Llama, Mistral) without per-token API costs
- **One-time cost**: ~$600-800 pays for itself in 3-6 months vs always-on cloud compute
- **No cold starts**: Always warm and ready

However, this approach has drawbacks:
- Requires network configuration (Tailscale/Cloudflare Tunnel) for remote access
- Hardware maintenance and power considerations
- Upfront capital expenditure

**Key insight**: If using API-based LLMs (Claude, GPT-4) anyway, the local inference argument disappears. The compute bottleneck is the API, not local processing.

#### Railway
- Simpler deployment UX, more "magic"
- Volume support is newer and less battle-tested
- Single region per service

#### External Database (CloudSQL, Neon, Supabase)
- Fully managed by cloud provider
- Mature backup/recovery options
- However: requires cross-network configuration, adds latency, more moving parts

#### Fly.io with Fly Postgres (Selected)
- Firecracker microVMs with mature volume support
- Multi-region deployment capability for lower latency
- Edge-first architecture
- First-class volume support (critical for git repos)
- **Fly Postgres**: Same network as app, automatic connection, simpler setup
- Good free tier ($5/month credit)

## Decision
Deploy Vibe Kanban to **Fly.io** with:

1. **Database**: Fly Postgres (same region as app, automatic networking)
2. **Persistent Volume**: Fly volume mounted at `/repos` for git worktrees
3. **Single Region**: Start with one region, expand if needed
4. **Auto-scaling**: Scale to zero when idle, wake on request

### Architecture
```
┌─────────────────────────────────────────────────────────────┐
│                         Internet                             │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                        Fly.io                                │
│                                                              │
│  ┌───────────────────────────────────────────────────────┐  │
│  │                  Fly Proxy (anycast)                   │  │
│  └───────────────────────────────────────────────────────┘  │
│                              │                               │
│         ┌────────────────────┴────────────────────┐         │
│         ▼                                         ▼         │
│  ┌─────────────────────────┐    ┌─────────────────────────┐ │
│  │   Vibe Kanban (microVM) │    │   Fly Postgres          │ │
│  │  ┌─────────┬──────────┐ │    │   (PostgreSQL 16)       │ │
│  │  │Rust API │ Frontend │ │◄──►│                         │ │
│  │  │ (Axum)  │ (Static) │ │    │   - Automatic backups   │ │
│  │  └─────────┴──────────┘ │    │   - Same-region latency │ │
│  │           │             │    └─────────────────────────┘ │
│  │           ▼             │                                │
│  │  ┌─────────────────────┐│                                │
│  │  │  /repos (Volume)    ││                                │
│  │  │  - Git worktrees    ││                                │
│  │  │  - Agent workdirs   ││                                │
│  │  └─────────────────────┘│                                │
│  └─────────────────────────┘                                │
└─────────────────────────────────────────────────────────────┘
```

### Configuration (`fly.toml`)
```toml
app = 'vibe-kanban-sm'
primary_region = 'sjc'  # San Jose

[build]
  dockerfile = 'Dockerfile'

[env]
  HOST = '0.0.0.0'
  RUST_LOG = 'info'

[http_service]
  internal_port = 3000
  force_https = true
  auto_stop_machines = 'stop'
  auto_start_machines = true
  min_machines_running = 0

[mounts]
  source = 'repos_data'
  destination = '/repos'
  initial_size = '10gb'

[[vm]]
  size = 'shared-cpu-1x'
  memory = '512mb'
```

### Environment Variables
| Variable | Source | Description |
|----------|--------|-------------|
| `DATABASE_URL` | Fly Postgres (auto-attached) | Connection string injected automatically |
| `RUST_LOG` | fly.toml env | Log level (info) |
| `HOST` | fly.toml env | Bind address (0.0.0.0) |

## Consequences

### Positive
- Access from anywhere without VPN/tunnel setup
- No local machine needs to stay running
- Auto-scaling reduces costs during idle periods
- Geographic flexibility for future multi-region
- Professional deployment with health checks, logs, metrics
- **Simplified networking**: Fly Postgres is on same internal network, no firewall config needed
- **Automatic DATABASE_URL**: Fly injects connection string when Postgres is attached

### Negative
- Monthly cost (~$5-15/month depending on usage)
- Cold start latency when scaling from zero (~2-5 seconds)
- Volume is single-region (data locality constraint)
- Fly Postgres is less "managed" than CloudSQL (you own the VM)

### Neutral
- Git operations happen on Fly volume, not local filesystem
- Debugging requires `fly ssh console` instead of local tools
- Logs accessed via `fly logs` instead of local stdout

## Implementation Plan

### Phase 1: Infrastructure Setup
1. **Install Fly CLI**
   ```bash
   brew install flyctl
   fly auth login
   ```

2. **Create Fly app**
   ```bash
   fly apps create vibe-kanban-sm  # or choose unique name
   ```

3. **Create Fly Postgres database**
   ```bash
   fly postgres create --name vibe-kanban-db --region sjc
   ```
   Select configuration when prompted (Development is fine to start).

4. **Attach Postgres to app**
   ```bash
   fly postgres attach vibe-kanban-db --app vibe-kanban-sm
   ```
   This automatically sets `DATABASE_URL` as a secret.

5. **Create persistent volume**
   ```bash
   fly volumes create repos_data --region sjc --size 10 --app vibe-kanban-sm
   ```

### Phase 2: Deploy
1. **Verify fly.toml** is in repository root with correct app name

2. **Initial deployment**
   ```bash
   fly deploy
   ```

3. **Verify health**
   ```bash
   fly status
   fly logs
   ```

4. **Test application**
   ```bash
   fly open  # Opens browser to deployed app
   ```

### Phase 3: Operational Setup
1. **Custom domain** (optional)
   ```bash
   fly certs add your-domain.com
   ```

2. **Monitoring**
   - Fly dashboard for metrics
   - Set up alerts for errors/downtime

3. **Database access**
   ```bash
   fly postgres connect --app vibe-kanban-db  # psql into database
   fly proxy 5432 --app vibe-kanban-db        # Local proxy for tools
   ```

4. **Backup strategy**
   - Fly Postgres has automatic daily snapshots
   - `fly postgres backup list --app vibe-kanban-db`
   - Consider periodic volume snapshots for repos

## Security Considerations

1. **Database Connection**
   - Fly Postgres uses internal networking (no public exposure)
   - Connection string stored as Fly secret, not in fly.toml

2. **Network Access**
   - Fly provides HTTPS by default
   - Postgres not exposed to internet (internal only)

3. **Volume Data**
   - Git repos on volume may contain sensitive code
   - Volume is encrypted at rest by Fly

## Rollback Plan
If deployment fails or causes issues:
1. `fly deploy --image <previous-image>` to rollback
2. Local development continues to work independently
3. Postgres data is unaffected by app deployment issues

## Useful Commands
```bash
# View logs
fly logs --app vibe-kanban-sm

# SSH into running machine
fly ssh console --app vibe-kanban-sm

# Check app status
fly status --app vibe-kanban-sm

# Database shell
fly postgres connect --app vibe-kanban-db

# View secrets
fly secrets list --app vibe-kanban-sm

# Scale (if needed)
fly scale memory 1024 --app vibe-kanban-sm
```

## Related
- ADR 2026-01-18-001: Structured Deliverables (agent workflow context)
- ADR 2026-01-18-002: Task Auto-start Triggers (requires always-on for triggers to fire)

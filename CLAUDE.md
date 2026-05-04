# Claude Code Instructions for BBA-Tools

## Architecture

BBA-Tools is a pure Rust project using Edward Piwowar's native EPBot libraries (NativeAOT-compiled .NET → native shared libraries). No .NET runtime needed at runtime.

### Components

| Directory | Purpose |
|-----------|---------|
| `epbot-core/` | Shared Rust crate: FFI bindings to native EPBot, auction orchestration, convention loading |
| `bba-cli/` | CLI binary (`bba-cli`): batch PBN processing |
| `bba-server/` | Axum web server (`bba-server`): REST API for browser extensions |
| `epbot-libs/` | Native EPBot libraries per platform (checked into repo) |
| `legacy/` | Retired C# code (`bba-server-cs`, `bba-cli-cs`, `epbot-wrapper`) and old Windows tooling, kept as reference. Not built by CI. |
| `history/` | Archived documentation from the Windows-hosted era |

### EPBot Native Libraries

From Edward Piwowar's NativeAOT build. Located in `epbot-libs/`:
- `linux/x64/libEPBot.so`, `linux/arm64/libEPBot.so`
- `macos/arm64/libEPBot.dylib`
- `windows/x64/EPBot.dll`, `windows/arm64/EPBot.dll` (Windows builds pending namespace fix)

## BBA Server (Production)

The Rust bba-server runs on a DigitalOcean droplet, behind Caddy reverse proxy.

### Server Details

| Item | Value |
|------|-------|
| Droplet IP | `146.190.135.172` |
| SSH | `ssh root@146.190.135.172` (Mac id_ed25519 key) |
| Public URL | `https://bba.harmonicsystems.com` |
| Install path | `/opt/bba-server/` |
| Systemd service | `bba-server` |
| Reverse proxy | Caddy at `/opt/livekit/Caddyfile` |
| DNS | Cloudflare A record → droplet IP (DNS only, Caddy handles TLS) |
| Also on droplet | LiveKit at `/opt/livekit/` (docker-compose) |

### Key Endpoints

- `GET /health` - Health check
- `POST /api/auction/generate` - Generate auction for a deal
- `GET /api/scenarios` - List available scenarios
- `POST /api/scenario/select` - Record scenario selection (analytics)

### Admin Dashboard

- `GET /admin/dashboard?key=<admin_key>` - Usage stats, charts, request history
- `GET /admin/whoami` - Debug endpoint showing detected IP and access status

Admin access via `?key=` query parameter. Admin users (for filtering): `Valerie_Perez`, `Travis_Scott`, `Tom_Martinez`, `Carol_Jordan`, `Joe_Evans`, `Rebecca_Coleman`, `Timothy_Carter`

The dashboard HTML is served from disk at `/opt/bba-server/wwwroot/dashboard.html` — editable without rebuilding the binary.

### Server Management

**Check status:**
```bash
ssh root@146.190.135.172 'systemctl status bba-server --no-pager'
```

**View logs:**
```bash
ssh root@146.190.135.172 'journalctl -u bba-server -n 50 --no-pager'
```

**Deploy new version** (after CI builds a release):
```bash
ssh root@146.190.135.172 'bash -s' << 'REMOTE'
systemctl stop bba-server
cd /opt/bba-server
curl -sL https://github.com/Rick-Wilson/BBA-Tools/releases/download/TAG/bba-TAG-linux-x64.tar.gz | tar xz
systemctl start bba-server
REMOTE
```

**Update dashboard only** (no rebuild needed):
```bash
scp bba-server/wwwroot/dashboard.html root@146.190.135.172:/opt/bba-server/wwwroot/
```

**Restart Caddy** (if Caddyfile changes):
```bash
ssh root@146.190.135.172 'cd /opt/livekit && docker compose restart caddy'
```

### Maintenance & Updates

Automatic reboots are disabled (`/etc/apt/apt.conf.d/51no-auto-reboot`). Unattended security upgrades still install but won't reboot.

**Important:** System library updates (especially `libssl3`) can break EPBot's NativeAOT runtime without warning. On 2026-04-09, an automatic `libssl3` update caused "Arithmetic operation resulted in an overflow" on all `epbot_create()` calls. A reboot fixed it.

**Before applying OS updates:**
1. Check for pending updates: `ssh root@146.190.135.172 'apt list --upgradable'`
2. Plan a maintenance window (low-traffic period)
3. Apply updates: `ssh root@146.190.135.172 'apt upgrade -y'`
4. Restart bba-server: `ssh root@146.190.135.172 'systemctl restart bba-server'`
5. Verify: `curl https://bba.harmonicsystems.com/health`
6. If EPBot fails, reboot: `ssh root@146.190.135.172 'reboot'`

**Check for pending reboot:** `ssh root@146.190.135.172 'cat /var/run/reboot-required 2>/dev/null || echo "no reboot required"'`

### Configuration

Environment file: `/opt/bba-server/.env`

```
HOST=0.0.0.0
PORT=5000
LOG_PATH=/opt/bba-server/logs
MAX_CONCURRENCY=4
DEFAULT_NS_CARD=21GF-DEFAULT
DEFAULT_EW_CARD=21GF-GIB
GITHUB_RAW_BASE_URL=https://raw.githubusercontent.com/ADavidBailey/Practice-Bidding-Scenarios/main
ADMIN_USERS=Valerie_Perez,Travis_Scott,Tom_Martinez,Carol_Jordan,Joe_Evans,Rebecca_Coleman,Timothy_Carter
ADMIN_KEY=goosebumps
```

Convention cards (.bbsa) and scenario files (.pbs) are fetched from GitHub at runtime.

### Logs

Logs are in `/opt/bba-server/logs/`:
- `audit-auction-YYYY-MM.csv` - Auction request audit log
- `audit-scenario-YYYY-MM.csv` - Scenario selection audit log

CSV columns (current format):
- Auction: `Timestamp,RequestIP,ClientVersion,Extension,Browser,OS,DurationMs,Version,EPBotVersion,Dealer,Vulnerability,Scoring,NSConvention,EWConvention,Scenario,PBN,Success,Auction,Alerts,Error`
- Scenario: `Timestamp,RequestIP,ClientVersion,Extension,Browser,OS,Version,Scenario`

### Client Info Header

Browser extensions send `X-Client-Info: ext=BBOAlert|PBSforBBO; browser=Chrome|Firefox|Safari|Edge; os=Windows|macOS|Linux` for environment tracking.

## Building

GitHub Actions (`.github/workflows/build.yml`) builds all platforms on push to main. Tagged releases (`v*`) create GitHub Releases.

### Local macOS build

```bash
# CLI
cd bba-cli && cargo build --release

# Server
cd bba-server && cargo build --release

# Run server locally
DYLD_LIBRARY_PATH=../epbot-libs/macos/arm64 cargo run
```

### Dependencies

- `epbot-core` depends on native EPBot library at link time
- `bba-cli` depends on `epbot-core` and `bridge-parsers` (sibling repo at `../../Bridge-Parsers`)
- `bba-server` depends on `epbot-core`

## Windows VM Access via SSH

The Windows VM is still used for testing Windows-specific EPBot functionality and the legacy C# components.

### SSH Runner

```python
import os, sys
os.environ['WINDOWS_HOST'] = '10.211.55.5'
os.environ['WINDOWS_USER'] = 'Rick'
sys.path.insert(0, '/Users/rick/Development/GitHub/Practice-Bidding-Scenarios/build-scripts-mac')
from ssh_runner import run_windows_command
```

### Drive Mappings

| Windows Drive | Mac Path |
|--------------|----------|
| `G:` | `/Users/rick/Development/GitHub` |
| `P:` | `/Users/rick/Development/GitHub/Practice-Bidding-Scenarios` |

### Convention Files

- Mac: `/Users/rick/Development/GitHub/Practice-Bidding-Scenarios/bbsa/`
- Windows: `P:\bbsa\`
- Default convention: `21GF-DEFAULT.bbsa`

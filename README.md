BBA-CLI

[![N|Solid](https://github.com/EdwardPiwowar/BBA/blob/main/BBALogo.jpg?raw=true)](https://sites.google.com/view/bbaenglish)

Cross-platform CLI and server for Bridge Bot Analyzer (EPBot)

## Architecture

BBA-CLI uses Edward Piwowar's native EPBot libraries (NativeAOT-compiled) via Rust FFI. No .NET runtime required.

| Component | Description |
|-----------|-------------|
| `epbot-core/` | Shared Rust crate: FFI bindings, auction orchestration, convention loading |
| `cli/` | CLI tool: batch-processes PBN files to generate auctions |
| `bba-server-rs/` | Axum web server: REST API for browser extensions |
| `epbot-libs/` | Native EPBot libraries for each platform |

### Platform Support

| Platform | CLI | Server | EPBot Library |
|----------|-----|--------|---------------|
| Linux x64 | Yes | Yes | `libEPBot.so` |
| Linux arm64 | Yes | Yes | `libEPBot.so` |
| macOS arm64 | Yes | Yes | `libEPBot.dylib` |
| Windows x64 | Yes | Yes | `EPBot.dll` (pending fix) |

## Links

- BBA Homepage: https://sites.google.com/view/bbaenglish
- EPBot API Documentation: https://sites.google.com/view/bbaenglish/for-programmers
- BBA Server: https://bba.harmonicsystems.com/health
- Admin Dashboard: https://bba.harmonicsystems.com/admin/dashboard

## CLI Usage

```
bba-cli --input hands.pbn --output auctions.pbn --ns-conventions 21GF-DEFAULT --ew-conventions 21GF-GIB
```

## Server API

- `GET /health` - Health check
- `POST /api/auction/generate` - Generate auction for a deal
- `GET /api/scenarios` - List available scenarios
- `POST /api/scenario/select` - Record scenario selection

## Building

GitHub Actions builds all platforms on push to main. Tagged releases (`v*`) create GitHub Releases with platform-specific archives.

```bash
# Local build (macOS)
cd cli && cargo build --release
cd bba-server-rs && cargo build --release
```

The EPBot native library must be available at runtime (in `epbot-libs/` or on the library path).

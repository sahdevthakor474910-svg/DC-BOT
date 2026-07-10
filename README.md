# 🤖 Discord Meme Bot (Rust)

A production-ready Discord bot written in **Rust** using the [Poise](https://github.com/serenity-rs/poise) slash-command framework and [Serenity](https://github.com/serenity-rs/serenity) library.

## ✨ Features

| Feature | Details |
|---|---|
| **Auto-reactions** | React to every message in configured channels |
| **User reactions** | React to messages from specific users with configurable emoji |
| **Reddit memes** | Fetches hot posts from r/memes, r/dankmemes, r/shitposting, r/brainrot, r/196 every 5 minutes |
| **Deduplication** | SQLite-backed post-ID tracking — no duplicate memes |
| **Media support** | Images, GIFs, videos (Reddit-hosted MP4s) |
| **Slash commands** | Full admin configuration via `/config` and `/memes` commands |
| **NSFW posts** | Included (not filtered) |
| **Docker** | Multi-stage build + `docker-compose.yml` for one-command deployment |

---

## 🚀 Quick Start

### Prerequisites

- [Rust](https://rustup.rs/) 1.75+
- A Discord bot token ([Developer Portal](https://discord.com/developers/applications))
- Reddit account (for the User-Agent — no OAuth needed)

### 1. Clone & Configure

```bash
git clone <your-repo>
cd dc-bot
cp .env.example .env
```

Edit `.env`:

```env
DISCORD_TOKEN=your_bot_token
DISCORD_CLIENT_ID=your_application_id
REDDIT_USER_AGENT=discord-meme-bot/1.0 (by /u/your_reddit_username)
DATABASE_URL=sqlite://data/bot.db
LOG_LEVEL=info
```

### 2. Run Locally

```bash
mkdir -p data
cargo run --release
```

### 3. Run with Docker

```bash
docker compose up -d
```

Logs:
```bash
docker compose logs -f dc-bot
```

---

## 🔧 Discord Developer Portal Setup

1. Go to [discord.com/developers/applications](https://discord.com/developers/applications)
2. Create a new Application → **Bot** tab
3. Enable **Privileged Gateway Intents**:
   - ✅ **Message Content Intent** (required for auto-reactions)
4. Copy the **Token** and **Application ID** into your `.env`
5. Invite the bot with these scopes:
   - `bot` + `applications.commands`
   - Required permissions: `Send Messages`, `Read Message History`, `Add Reactions`, `View Channel`

Invite URL template:
```
https://discord.com/api/oauth2/authorize?client_id=YOUR_CLIENT_ID&permissions=274878024768&scope=bot%20applications.commands
```

---

## 📋 Slash Commands

### `/ping`
Check bot latency.

---

### `/config` *(requires Manage Server)*

| Command | Description |
|---|---|
| `/config meme-channel #channel` | Set the channel where memes are posted |
| `/config add-reaction-channel #channel` | Auto-react to every message in this channel |
| `/config remove-reaction-channel #channel` | Remove a reaction channel |
| `/config add-user @user` | Auto-react to messages from this user |
| `/config remove-user @user` | Remove a user from auto-react list |
| `/config add-emoji 🔥` | Add an emoji to the reaction list |
| `/config remove-emoji 🔥` | Remove an emoji from the reaction list |
| `/config interval 300` | Set meme posting interval (seconds, min 60) |
| `/config show` | Display current server configuration |

> **Custom emojis** are supported. Use the format `<:name:id>` or `<a:name:id>` for animated emojis.

---

### `/memes`

| Command | Description |
|---|---|
| `/memes status` | Show task status, configured channel, and subreddits |
| `/memes fetch-now` | Manually trigger an immediate meme fetch *(requires Manage Server)* |

---

## 🗂️ Project Structure

```
src/
├── main.rs            # Entry point: Poise framework, event dispatch, task spawn
├── config.rs          # Environment variable loading
├── data.rs            # Shared Data struct + type aliases
├── db/
│   ├── mod.rs
│   ├── schema.rs      # Startup migration runner
│   └── queries.rs     # All SQL query functions
├── reddit/
│   ├── mod.rs
│   ├── models.rs      # Serde structs for Reddit JSON
│   ├── client.rs      # reqwest HTTP client + media URL resolution
│   └── task.rs        # Background async task (fetch + post loop)
├── events/
│   ├── mod.rs
│   └── message.rs     # on_message: auto-react logic
└── commands/
    ├── mod.rs         # Command registry
    ├── ping.rs        # /ping
    ├── config.rs      # /config (all subcommands)
    └── memes.rs       # /memes status & fetch-now
```

---

## 🐳 Docker Deployment (Production)

The `Dockerfile` uses a two-stage build:

1. **Builder** — compiles the bot with `cargo build --release`
2. **Runtime** — `debian:bookworm-slim` with only CA certs and OpenSSL

SQLite data is stored in a mounted volume (`./data/`).

```bash
# Build image
docker compose build

# Start (detached)
docker compose up -d

# View logs
docker compose logs -f

# Stop
docker compose down
```

---

## ⚙️ Environment Variables

| Variable | Required | Default | Description |
|---|---|---|---|
| `DISCORD_TOKEN` | ✅ | — | Bot token from Developer Portal |
| `DISCORD_CLIENT_ID` | ✅ | — | Application ID |
| `DATABASE_URL` | ❌ | `sqlite://data/bot.db` | SQLite path |
| `REDDIT_USER_AGENT` | ❌ | `discord-meme-bot/1.0` | Reddit API User-Agent |
| `LOG_LEVEL` | ❌ | `info` | `trace`, `debug`, `info`, `warn`, `error` |

---

## 🛡️ Error Handling

- All commands return descriptive error messages in Discord on failure
- Background task errors are logged and the loop continues (no crash)
- Old seen-post records are pruned every 30 days to keep the DB small

---

## 📜 License

MIT

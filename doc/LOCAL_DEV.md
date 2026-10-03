# Local development

Run everything on the laptop, rebuild on Railway only to ship. The cloud
rebuilds incrementally (~10-60 s per change vs ~20-30 min on Railway).

## Prerequisites

- Rust 1.86 via rustup (`rust-toolchain.toml` pins it), `protoc` (`brew install protobuf`)
- Docker for the infra containers (`docker compose` subcommand present)
- Node + pnpm 10 for the web

## First run

```bash
./dev.sh init                  # creates .env.local (gitignored) with local secrets
./dev.sh infra                 # postgres(:5433, pg16+pgvector) + redis + gotrue:0.9.64
```

Then, one terminal per service:

```bash
./dev.sh cloud                 # appflowy_cloud on :8000, hot reload via cargo watch
./dev.sh admin                 # admin frontend on :3001 -> /web
./dev.sh web                   # Vite on :5173, same-origin proxies, HMR
```

## First user

Signups are open locally (`LOCAL_GOTRUE_DISABLE_SIGNUP=false`, mail
auto-confirmed), so sign up directly in the web UI — but the era web's
password flow skips `/api/user/verify`, leaving the account without profile or
workspace. Either log in once via magic-link/OAuth flow (not available without
SMTP) or just run:

```bash
./dev.sh seed dev-amy@example.com [password]   # create + confirm + verify
```

## Daily loop

1. Edit Rust in this repo → `cargo watch` rebuilds and restarts `:8000` by itself.
2. Edit the web in `../AppFlowy/appflowy-web/` → Vite HMR, no rebuild.
3. Check health: `http://localhost:8000/health`, gotrue at `:9999`.
4. Happy with it? Sync and ship: `./../AppFlowy/rebuild.sh appflowy-cloud`.

## Gotchas

- **Ports**: postgres is on **5433** and the web on **5173** — the machine's own
  postgres (5432) and node (3000) were already taken when this was set up.
- **File storage goes to R2**, not local MinIO (dead upstream). It uses the
  Railway R2 credentials with a **separate bucket** (`R2_BUCKET_DEV`, default
  `appflowy-dev`) so dev never touches production files. The bucket
  `appflowy-dev` already exists in the account — this matters because the
  era cloud reads/writes collabs through S3, and a missing bucket breaks
  workspace init and the folder endpoint (`NoSuchBucket`), leaving the web
  stuck on skeleton placeholders.
- **Migrations run at cloud startup** (`sqlx::migrate!`), so `./dev.sh reset`
  (wipe + restart) always gives a clean migrated database.
- **Vite proxy** (`appflowy-web/vite.config.ts`) keeps the browser same-origin,
  so no CORS is involved; it only affects `pnpm dev`, never the production build.
- **SQLX offline**: the repo ships `.sqlx/` so `cargo run` needs no live DB at
  compile time. Only runtime `QueryBuilder` queries were touched by the fork
  patch, so the offline cache stays valid.
- **Gotcha, admin user**: `LOCAL_GOTRUE_ADMIN_EMAIL` is created as a system
  admin by gotrue's start script — like prod, it cannot use the app itself.

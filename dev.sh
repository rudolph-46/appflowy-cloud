#!/usr/bin/env bash
#
# Local development loop — iterate on localhost, rebuild on Railway only to ship.
#
#   ./dev.sh init                 create .env.local (once)
#   ./dev.sh infra                start postgres+redis+gotrue (docker compose)
#   ./dev.sh cloud                run appflowy_cloud with hot reload (cargo watch)
#   ./dev.sh admin                run admin_frontend (localhost:3001 -> /web)
#   ./dev.sh web                  run the Vite dev server (localhost:5173, proxied)
#   ./dev.sh seed <email> [pass]  create+confirm+verify a local user
#   ./dev.sh reset                wipe the local database (down -v + up)
#
# The cloud rebuilds incrementally (~10-60s per change); Railway rebuilds are
# only needed for deployment (see ../AppFlowy/rebuild.sh).

set -euo pipefail
cd "$(dirname "$0")"

COMPOSE="docker compose -f docker-compose.local.yml"
RAILWAY_DIR="../AppFlowy"

need_env() {
  [[ -f .env.local ]] || { echo "missing .env.local — run ./dev.sh init first"; exit 1; }
}

load_env() {
  need_env
  set -a; . ./.env.local; set +a
}

cloud_env() {
  load_env
  # R2 comes from the Railway secrets (same account), but a SEPARATE bucket
  # so dev never touches production files.
  if [[ -f $RAILWAY_DIR/.env.railway ]]; then set -a; . "$RAILWAY_DIR/.env.railway"; set +a; fi
  if [[ -z ${R2_BUCKET_DEV:-} ]]; then
    R2_BUCKET_DEV=appflowy-dev
    echo "note: R2_BUCKET_DEV unset — using appflowy-dev (create it in the R2 dashboard for uploads to work)"
  fi
  : "${R2_ACCOUNT_ID:?missing R2 creds in $RAILWAY_DIR/.env.railway}"
  local S3_URL="https://$R2_ACCOUNT_ID.r2.cloudflarestorage.com"
  export SQLX_OFFLINE=true
  export RUST_LOG=${RUST_LOG:-info}
  export APPFLOWY_ENVIRONMENT=local
  export APPFLOWY_APPLICATION_HOST=0.0.0.0
  export APPFLOWY_APPLICATION_PORT=8000
  export APPFLOWY_DATABASE_URL="postgres://$LOCAL_POSTGRES_USER:$LOCAL_POSTGRES_PASSWORD@localhost:$LOCAL_POSTGRES_PORT/$LOCAL_POSTGRES_DB"
  export APPFLOWY_DATABASE_MAX_CONNECTIONS=10
  export APPFLOWY_REDIS_URI="redis://localhost:$LOCAL_REDIS_PORT"
  export APPFLOWY_GOTRUE_JWT_SECRET="$LOCAL_GOTRUE_JWT_SECRET"
  export APPFLOWY_GOTRUE_BASE_URL=http://localhost:9999
  export APPFLOWY_ACCESS_CONTROL=true
  export APPFLOWY_S3_USE_MINIO=true
  export APPFLOWY_S3_CREATE_BUCKET=false
  export APPFLOWY_S3_MINIO_URL="$S3_URL"
  export APPFLOWY_S3_ACCESS_KEY="$R2_ACCESS_KEY_ID"
  export APPFLOWY_S3_SECRET_KEY="$R2_SECRET_ACCESS_KEY"
  export APPFLOWY_S3_BUCKET="$R2_BUCKET_DEV"
  export APPFLOWY_S3_REGION=auto
  export APPFLOWY_S3_PRESIGNED_URL_ENDPOINT="$S3_URL"
  export APPFLOWY_BASE_URL=http://localhost:8000
  export APPFLOWY_WEB_URL=http://localhost:5173
  export AI_ENABLED=false
}

cmd=${1:-}; shift 2>/dev/null || true
case $cmd in
  init)
    [[ -f .env.local ]] && { echo ".env.local exists — kept"; exit 0; }
    gen() { openssl rand -hex 24; }
    cat > .env.local <<EOF
LOCAL_POSTGRES_USER=postgres
LOCAL_POSTGRES_PASSWORD=$(gen)
LOCAL_POSTGRES_DB=postgres
LOCAL_POSTGRES_PORT=5433
LOCAL_REDIS_PORT=6379
LOCAL_GOTRUE_JWT_SECRET=$(gen)
LOCAL_GOTRUE_ADMIN_EMAIL=admin@example.com
LOCAL_GOTRUE_ADMIN_PASSWORD=$(gen)
LOCAL_GOTRUE_DISABLE_SIGNUP=false
RUST_LOG=info
EOF
    echo ".env.local created"
    ;;

  infra)
    load_env
    $COMPOSE up -d
    $COMPOSE ps
    ;;

  cloud)
    cloud_env
    echo "appflowy_cloud on http://localhost:8000 (hot reload via cargo watch)"
    exec cargo watch -x run
    ;;

  cloud-once)
    cloud_env
    echo "appflowy_cloud on http://localhost:8000 (single run, no watch)"
    exec cargo run -p appflowy-cloud
    ;;

  admin)
    load_env
    export RUST_LOG=${RUST_LOG:-info}
    export ADMIN_FRONTEND_HOST=0.0.0.0
    export ADMIN_FRONTEND_PORT=3001
    export ADMIN_FRONTEND_REDIS_URL="redis://localhost:$LOCAL_REDIS_PORT"
    export ADMIN_FRONTEND_GOTRUE_URL=http://localhost:9999
    export ADMIN_FRONTEND_APPFLOWY_CLOUD_URL=http://localhost:8000
    export ADMIN_FRONTEND_PATH_PREFIX=""
    echo "admin frontend on http://localhost:3001 -> /web"
    exec cargo run -p admin_frontend
    ;;

  web)
    cd "$RAILWAY_DIR/appflowy-web"
    echo "web dev server on http://localhost:5173 (proxied to local cloud/gotrue)"
    exec env PORT=5173 pnpm dev
    ;;

  seed)
    email=${1:?usage: dev.sh seed <email> [password]}
    pass=${2:-}
    if [[ -z $pass ]]; then pass=$(openssl rand -base64 15 | tr '+/' 'Ax'); fi
    load_env
    atok=$(curl -s -X POST "http://localhost:9999/token?grant_type=password" \
      -H "Content-Type: application/json" \
      -d "{\"email\":\"$LOCAL_GOTRUE_ADMIN_EMAIL\",\"password\":\"$LOCAL_GOTRUE_ADMIN_PASSWORD\"}" \
      | python3 -c 'import json,sys; print(json.load(sys.stdin).get("access_token",""))')
    [[ -n $atok ]] || { echo "admin login failed — is gotrue up (./dev.sh infra)?"; exit 1; }
    out=$(curl -s -X POST "http://localhost:9999/admin/users" \
      -H "Authorization: Bearer $atok" -H "Content-Type: application/json" \
      -d "{\"email\":\"$email\",\"password\":\"$pass\",\"email_confirm\":true}")
    tok=$(curl -s -X POST "http://localhost:9999/token?grant_type=password" \
      -H "Content-Type: application/json" \
      -d "{\"email\":\"$email\",\"password\":\"$pass\"}" \
      | python3 -c 'import json,sys; print(json.load(sys.stdin).get("access_token",""))')
    [[ -n $tok ]] || { echo "user login failed: $out"; exit 1; }
    curl -s "http://localhost:8000/api/user/verify/$tok" -H "Authorization: Bearer $tok" >/dev/null
    echo "seeded: $email"
    echo "password: $pass"
    ;;

  down) $COMPOSE down ;;
  reset) $COMPOSE down -v; load_env; $COMPOSE up -d;;

  *)
    echo "usage: dev.sh init | infra | cloud | cloud-once | admin | web | seed <email> [pass] | down | reset"
    exit 1
    ;;
esac

#!/bin/sh
# Prepare config/master.key on first run so `docker compose up` needs no
# manual editing. All paths point at the compose `mysql` service by name.

set -e

CONFIG_DIR=/app/config
KEY_FILE="$CONFIG_DIR/master.key"
CFG_FILE="$CONFIG_DIR/config.toml"

mkdir -p "$CONFIG_DIR" /app/data

if [ ! -s "$KEY_FILE" ]; then
    echo "[entrypoint] generating 32-byte master.key"
    head -c 32 /dev/urandom > "$KEY_FILE"
    chmod 600 "$KEY_FILE" || true
fi

if [ ! -s "$CFG_FILE" ]; then
    echo "[entrypoint] writing default config.toml for docker compose"
    cat > "$CFG_FILE" <<'EOF'
[server]
bind_address = "0.0.0.0"
port = 8000

[database]
url = "mysql://hub_user:hub_pass@mysql:3306/field_service_hub"

[encryption]
key_file = "/app/config/master.key"

[storage]
review_images_dir      = "/app/data/review_images"
report_attachments_dir = "/app/data/report_attachments"
face_images_dir        = "/app/data/face_images"

[policy]
local_timezone_offset_minutes = 0
late_grace_hours              = 72
EOF
fi

exec /app/backend

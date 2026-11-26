#!/bin/sh
set -eu
umask 077
: "${XINAPI_CPR_INITIAL_PASSWORD:?}"
# Render only the upstream admin bootstrap field. Store credentials arrive
# separately through the CPR_DATABASE_PASSWORD and CPR_REDIS_PASSWORD inputs.
awk '
  /^  default_password:/ {
    value=ENVIRON["XINAPI_CPR_INITIAL_PASSWORD"]
    if (value !~ /^[0-9a-f]+$/ || length(value) < 24) exit 2
    print "  default_password: \047" value "\047"; seen++; next
  }
  { print }
  END { if (seen != 1) exit 2 }
' /etc/xinapi/cpr-template.yaml > /app/deploy/config.yaml
unset XINAPI_CPR_INITIAL_PASSWORD
exec /app/bin/codex-proxy-rs

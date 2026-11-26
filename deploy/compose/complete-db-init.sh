#!/bin/sh
set -eu
: "${PGDATA:?}" "${POSTGRES_USER:?}" "${POSTGRES_PASSWORD:?}"
: "${NEWAPI_DB_PASSWORD:?}" "${CPR_DB_PASSWORD:?}"

if [ ! -s "$PGDATA/PG_VERSION" ]; then
  echo 'Use the normal startup command for an empty cluster.' >&2
  exit 1
fi
if [ -f "$PGDATA/.xinapi-databases-ready" ]; then
  echo 'Initialization is complete; run tools/start_compose.py to verify services.'
  exit 0
fi
export PGPASSWORD="$POSTGRES_PASSWORD"

# 先辨识已有对象的归属，再补齐缺失对象；数据保持在原卷。
psql -X -q -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d postgres <<'SQL'
DO $check$
BEGIN
  IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname IN ('newapi', 'cpr')
      AND (rolsuper OR rolcreatedb OR rolcreaterole OR rolreplication OR rolbypassrls)) THEN
    RAISE EXCEPTION 'Application roles require ordinary database privileges; review existing role grants.';
  END IF;
  IF EXISTS (SELECT 1 FROM pg_database d JOIN pg_roles r ON r.oid = d.datdba
      WHERE d.datname IN ('newapi', 'cpr') AND r.rolname <> d.datname) THEN
    RAISE EXCEPTION 'Application database ownership needs operator review; retain the volume.';
  END IF;
END
$check$;
SELECT format('CREATE ROLE %I NOLOGIN', name)
FROM (VALUES ('newapi'), ('cpr')) AS expected(name)
WHERE NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = name) \gexec
SELECT format('CREATE DATABASE %I OWNER %I', name, name)
FROM (VALUES ('newapi'), ('cpr')) AS expected(name)
WHERE NOT EXISTS (SELECT 1 FROM pg_database WHERE datname = name) \gexec
SQL

# 已启用角色先用原配置认证，确认口令匹配后保留其现有口令。
for role in newapi cpr; do
  can_login=$(psql -X -At -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d postgres \
    -c "SELECT rolcanlogin FROM pg_roles WHERE rolname='$role'")
  case "$role" in
    newapi) app_password=$NEWAPI_DB_PASSWORD ;;
    cpr) app_password=$CPR_DB_PASSWORD ;;
  esac
  if [ "$can_login" = t ]; then
    if ! PGPASSWORD="$app_password" psql -X -q -v ON_ERROR_STOP=1 -U "$role" -d "$role" -c 'SELECT 1' >/dev/null 2>&1; then
      echo "Existing $role authentication failed; supply its original configuration before continuing." >&2
      exit 1
    fi
  fi
done

# 两库授权齐全后，统一启用首次登录；已有登录角色的口令保持原值。
psql -X -q -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d postgres <<'SQL'
\getenv newapi_secret NEWAPI_DB_PASSWORD
\getenv cpr_secret CPR_DB_PASSWORD
BEGIN;
REVOKE ALL ON DATABASE newapi FROM PUBLIC, cpr;
REVOKE ALL ON DATABASE cpr FROM PUBLIC, newapi;
REVOKE CONNECT ON DATABASE postgres FROM PUBLIC;
REVOKE CONNECT ON DATABASE template1 FROM PUBLIC;
SELECT format('ALTER ROLE newapi LOGIN PASSWORD %L', :'newapi_secret')
FROM pg_roles WHERE rolname = 'newapi' AND NOT rolcanlogin \gexec
SELECT format('ALTER ROLE cpr LOGIN PASSWORD %L', :'cpr_secret')
FROM pg_roles WHERE rolname = 'cpr' AND NOT rolcanlogin \gexec
COMMIT;
SQL

for role in newapi cpr; do
  case "$role" in
    newapi) app_password=$NEWAPI_DB_PASSWORD; other=cpr ;;
    cpr) app_password=$CPR_DB_PASSWORD; other=newapi ;;
  esac
  PGPASSWORD="$app_password" psql -X -q -v ON_ERROR_STOP=1 -U "$role" -d "$role" -c 'SELECT 1' >/dev/null
  if PGPASSWORD="$app_password" psql -X -q -v ON_ERROR_STOP=1 -U "$role" -d "$other" -c 'SELECT 1' >/dev/null 2>&1; then
    echo "Cross-database access remains for $role; review role memberships and retain the volume." >&2
    exit 1
  fi
done

: > "$PGDATA/.xinapi-databases-ready"
echo 'Application databases authenticated with isolated access; initialization completed on the retained volume.'

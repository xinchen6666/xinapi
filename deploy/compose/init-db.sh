#!/bin/sh
set -eu
: "${POSTGRES_USER:?}" "${NEWAPI_DB_PASSWORD:?}" "${CPR_DB_PASSWORD:?}"
export PGPASSWORD="${POSTGRES_PASSWORD:?}"
# Enable both logins together, after both databases and ACLs exist.
psql -X -q -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d postgres <<'SQL'
\getenv newapi_pw NEWAPI_DB_PASSWORD
\getenv cpr_pw CPR_DB_PASSWORD
CREATE ROLE newapi NOLOGIN;
CREATE ROLE cpr NOLOGIN;
CREATE DATABASE newapi OWNER newapi;
CREATE DATABASE cpr OWNER cpr;
BEGIN;
REVOKE ALL ON DATABASE newapi FROM PUBLIC;
REVOKE ALL ON DATABASE cpr FROM PUBLIC;
REVOKE CONNECT ON DATABASE postgres FROM PUBLIC;
REVOKE CONNECT ON DATABASE template1 FROM PUBLIC;
ALTER ROLE newapi LOGIN PASSWORD :'newapi_pw';
ALTER ROLE cpr LOGIN PASSWORD :'cpr_pw';
COMMIT;
SQL
: > "${PGDATA:?}/.xinapi-databases-ready"

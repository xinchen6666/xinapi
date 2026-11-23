#!/usr/bin/env python3
"""Start the five-service Compose project and verify authenticated readiness once."""

import argparse
import json
import subprocess
import sys
import time
from urllib.parse import parse_qs, unquote, urlsplit


class StartupFailure(Exception):
    pass


def run(command, deadline, *, input_text=None, check_timeout=None):
    remaining = deadline - time.monotonic()
    if remaining <= 0:
        raise StartupFailure("compose: startup deadline reached")
    timeout = min(remaining, check_timeout) if check_timeout else remaining
    try:
        return subprocess.run(
            command,
            input=input_text,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=timeout,
            check=False,
        )
    except subprocess.TimeoutExpired:
        if time.monotonic() >= deadline or check_timeout is None:
            raise StartupFailure("compose: startup deadline reached") from None
        return None
    except OSError:
        raise StartupFailure("compose: command execution failed") from None


def checks_for(services, compose):
    checks = []
    for service, role, url_name, password_name in (
        ("new-api", "newapi", "SQL_DSN", None),
        ("cpr", "cpr", "CPR_DATABASE_URL", "CPR_DATABASE_PASSWORD"),
    ):
        environment = services[service]["environment"]
        uri = urlsplit(environment[url_name])
        if (
            uri.scheme not in ("postgres", "postgresql")
            or unquote(uri.username or "") != role
            or unquote(uri.path.lstrip("/")) != role
            or not uri.hostname
        ):
            raise ValueError("application database configuration")
        password = (
            environment[password_name]
            if password_name
            else unquote(uri.password or "")
        )
        sslmode = parse_qs(uri.query).get("sslmode", ["prefer"])[0]
        if not password or any("\n" in value for value in (password, sslmode)):
            raise ValueError("application database credential")
        sql = (
            f"SELECT current_user = '{role}' AND current_database() = '{role}' "
            "AND NOT rolsuper AND NOT rolcreatedb AND NOT rolcreaterole "
            "AND NOT rolreplication AND NOT rolbypassrls "
            "FROM pg_roles WHERE rolname = current_user"
        )
        checks.append(
            (
                "postgres/" + service,
                compose
                + [
                    "exec", "-T", "postgres", "sh", "-ec",
                    'IFS= read -r PGPASSWORD; IFS= read -r PGSSLMODE; '
                    'PGCONNECT_TIMEOUT=2; PGOPTIONS="-c statement_timeout=2000"; '
                    'export PGPASSWORD PGSSLMODE PGCONNECT_TIMEOUT PGOPTIONS; '
                    'exec timeout 3 "$@"',
                    "start-compose", "psql", "-X", "-w", "-v", "ON_ERROR_STOP=1",
                    "-h", uri.hostname, "-p", str(uri.port or 5432), "-U", role,
                    "-d", role, "-Atqc", sql,
                ],
                password + "\n" + sslmode + "\n",
                "t",
            )
        )
    for service, redis_service, url_name, password_name in (
        ("new-api", "redis-newapi", "REDIS_CONN_STRING", None),
        ("cpr", "redis-cpr", "CPR_REDIS_URL", "CPR_REDIS_PASSWORD"),
    ):
        environment = services[service]["environment"]
        uri = urlsplit(environment[url_name])
        if uri.scheme != "redis" or not uri.hostname:
            raise ValueError("application Redis configuration")
        password = (
            environment[password_name]
            if password_name
            else unquote(uri.password or "")
        )
        if not password or "\n" in password:
            raise ValueError("application Redis credential")
        command = compose + [
            "exec", "-T", redis_service, "sh", "-ec",
            'IFS= read -r REDISCLI_AUTH; export REDISCLI_AUTH; exec timeout 3 "$@"',
            "start-compose", "redis-cli", "--raw", "--no-auth-warning",
            "-h", uri.hostname, "-p", str(uri.port or 6379),
        ]
        if uri.username:
            command += ["--user", unquote(uri.username)]
        command += ["PING"]
        checks.append((redis_service, command, password + "\n", "PONG"))
    checks.extend(
        [
            (
                "new-api",
                compose
                + [
                    "exec", "-T", "new-api", "wget", "--quiet", "--output-document=-",
                    "--timeout=2", "--tries=1", "http://127.0.0.1:3000/api/status",
                ],
                None,
                "status-json",
            ),
            (
                "cpr",
                compose
                + [
                    "exec", "-T", "cpr", "curl", "--silent", "--fail",
                    "--max-time", "2", "--output", "/dev/null", "--write-out",
                    "%{http_code}", "http://127.0.0.1:8080/readyz",
                ],
                None,
                "204",
            ),
        ]
    )
    return checks


def main(argv=None):
    parser = argparse.ArgumentParser(
        description=__doc__,
        epilog="Run from the same directory as docker compose; its environment applies.",
    )
    parser.add_argument("--timeout", type=int, default=120, help="total timeout in seconds (default: 120)")
    parser.add_argument("--project-name", "-p", help="Compose project name")
    parser.add_argument("--wait-only", action="store_true", help="verify the existing project")
    parser.add_argument("--build", action="store_true", help="pass --build to compose up")
    args = parser.parse_args(argv)
    if args.timeout <= 0:
        parser.error("--timeout must be a positive number of seconds")
    compose = ["docker", "compose"]
    if args.project_name:
        compose += ["--project-name", args.project_name]
    deadline = time.monotonic() + args.timeout
    try:
        print("compose: checking configuration", flush=True)
        result = run(compose + ["config", "--format", "json"], deadline)
        if result.returncode:
            raise StartupFailure("compose: configuration failed")
        try:
            services = json.loads(result.stdout)["services"]
            checks = checks_for(services, compose)
        except (KeyError, TypeError, ValueError):
            raise StartupFailure("compose: application configuration is invalid") from None
        if not args.wait_only:
            print("compose: starting", flush=True)
            command = compose + ["up", "-d"]
            if args.build:
                command.append("--build")
            if run(command, deadline).returncode:
                raise StartupFailure("compose: startup command failed")
        states = {}
        while True:
            ready = True
            database_ready = True
            for name, command, input_text, expected in checks:
                result = run(command, deadline, input_text=input_text, check_timeout=4)
                success = result is not None and result.returncode == 0
                if success and expected == "status-json":
                    try:
                        data = json.loads(result.stdout)
                        success = isinstance(data, dict) and data.get("success") is True
                    except ValueError:
                        success = False
                elif success:
                    success = result.stdout.strip() == expected
                    if expected == "PONG":
                        success = success and not result.stderr.strip()
                ready = ready and success
                if name.startswith("postgres/"):
                    database_ready = database_ready and success
                if states.get(name) is not success:
                    print(name + (": ready" if success else ": waiting"), flush=True)
                    states[name] = success
            if ready:
                if time.monotonic() >= deadline:
                    raise StartupFailure("compose: startup deadline reached")
                print("compose: all five services ready", flush=True)
                return 0
            if not database_ready:
                result = run(
                    compose + ["logs", "--no-color", "--tail", "20", "postgres"],
                    deadline,
                    check_timeout=4,
                )
                if result is not None and "Incomplete xinapi cluster initialization" in result.stdout:
                    raise StartupFailure("postgres: incomplete initialization")
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise StartupFailure("compose: startup deadline reached")
            time.sleep(min(1, remaining))
    except (StartupFailure, KeyboardInterrupt) as error:
        status = str(error) if isinstance(error, StartupFailure) else "compose: interrupted"
        print(status, file=sys.stderr)
        print(
            "Preserve project volumes and original credentials. "
            "See docs/COMPOSE.md for diagnosis and preserved-volume repair.",
            file=sys.stderr,
        )
        return 1


if __name__ == "__main__":
    sys.exit(main())

"""Initialize private, service-specific Compose environment files once."""

import argparse
import os
from pathlib import Path
import secrets
import stat
from urllib.parse import quote, urlunsplit


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--directory", type=Path, default=Path(".runtime/x04"))
    directory = parser.parse_args().directory
    secrets_directory = directory / "new-api-secrets"
    required = {
        "postgres": (
            "POSTGRES_USER",
            "POSTGRES_DB",
            "POSTGRES_PASSWORD",
            "NEWAPI_DB_PASSWORD",
            "CPR_DB_PASSWORD",
        ),
        "redis-newapi": ("REDIS_PASSWORD",),
        "redis-cpr": ("REDIS_PASSWORD",),
        "new-api": ("SQL_DSN", "REDIS_CONN_STRING", "SESSION_SECRET", "CRYPTO_SECRET"),
        "cpr": (
            "CPR_DATABASE_URL",
            "CPR_DATABASE_PASSWORD",
            "CPR_REDIS_URL",
            "CPR_REDIS_PASSWORD",
            "XINAPI_CPR_INITIAL_PASSWORD",
        ),
    }
    os.umask(0o077)
    directory.parent.mkdir(parents=True, exist_ok=True)
    try:
        directory.mkdir(mode=0o700)
    except FileExistsError:
        if (
            directory.is_symlink()
            or not directory.is_dir()
            or directory.stat().st_mode & 0o077
        ):
            raise SystemExit("Use a private local configuration directory (0700).")
        for name, fields in required.items():
            path = directory / (name + ".env")
            if (
                not path.exists()
                or not stat.S_ISREG(path.lstat().st_mode)
                or path.stat().st_mode & 0o077
                or not path.stat().st_size
            ):
                raise SystemExit(
                    "Incomplete or public configuration; preserve existing files and resolve before startup."
                )
            values = dict(
                line.split("=", 1)
                for line in path.read_text().splitlines()
                if "=" in line
            )
            if any(not values.get(field, "").strip() for field in fields):
                raise SystemExit(
                    f"Missing required configuration in {name}.env; retain existing credentials."
                )
        secrets_directory.mkdir(mode=0o700, exist_ok=True)
        if (
            secrets_directory.is_symlink()
            or not secrets_directory.is_dir()
            or secrets_directory.stat().st_mode & 0o077
        ):
            raise SystemExit("Use a private backend key directory (0700).")
        print("Existing private configuration retained.")
        return

    def key():
        return secrets.token_hex(24)

    pg, na, cp, rn, rc = (key() for _ in range(5))
    envs = {
        "postgres": dict(
            POSTGRES_USER="postgres",
            POSTGRES_DB="postgres",
            POSTGRES_PASSWORD=pg,
            NEWAPI_DB_PASSWORD=na,
            CPR_DB_PASSWORD=cp,
        ),
        "redis-newapi": dict(REDIS_PASSWORD=rn),
        "redis-cpr": dict(REDIS_PASSWORD=rc),
        "new-api": dict(
            SQL_DSN=urlunsplit(
                (
                    "postgresql",
                    f"newapi:{quote(na, safe='')}@postgres:5432",
                    "/newapi",
                    "sslmode=disable",
                    "",
                )
            ),
            REDIS_CONN_STRING=urlunsplit(
                ("redis", f":{quote(rn, safe='')}@redis-newapi:6379", "/0", "", "")
            ),
            SESSION_SECRET=key(),
            CRYPTO_SECRET=key(),
            TRUSTED_PROXIES="none",
        ),
        "cpr": dict(
            CPR_DATABASE_URL="postgres://cpr@postgres:5432/cpr",
            CPR_DATABASE_PASSWORD=cp,
            CPR_REDIS_URL="redis://redis-cpr:6379/",
            CPR_REDIS_PASSWORD=rc,
            XINAPI_CPR_INITIAL_PASSWORD=key(),
        ),
    }
    for name in required:
        with (directory / (name + ".env")).open("x", encoding="utf-8") as handle:
            for field, value in envs[name].items():
                handle.write(f"{field}={value}\n")
    secrets_directory.mkdir(mode=0o700)
    print("Created private configuration for five Compose services.")


if __name__ == "__main__":
    main()

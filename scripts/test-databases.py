#!/usr/bin/env python3
"""Test schema generation with isolated servers and matching dump clients."""

import argparse
import copy
import json
import os
from pathlib import Path
import shlex
import subprocess
import tempfile


FEATURES = [
    "sqlite,sqlx,runtime-async-std,macros",
    "sqlite,diesel,runtime-async-std,macros",
    "sqlite,diesel",
    "mysql,diesel",
    "mysql,sqlx,runtime-async-std",
    "postgres,diesel",
    "postgres,sqlx,runtime-async-std",
    "sqlite,sqlx,runtime-tokio,macros",
    "sqlite,diesel,runtime-tokio,macros",
    "mysql,sqlx,runtime-tokio,macros",
    "mysql,diesel,runtime-tokio,macros",
    "postgres,sqlx,runtime-tokio,macros",
    "postgres,diesel,runtime-tokio,macros",
    "sqlite,sqlx,runtime-tokio,runtime-async-std,macros",
    "sqlite,diesel,runtime-tokio,runtime-async-std,macros",
    "mysql,sqlx,runtime-tokio,runtime-async-std,macros",
    "mysql,diesel,runtime-tokio,runtime-async-std,macros",
    "postgres,sqlx,runtime-tokio,runtime-async-std,macros",
    "postgres,diesel,runtime-tokio,runtime-async-std,macros",
]


def run(command, root, env):
    print("+ " + shlex.join(command), flush=True)
    subprocess.run(command, cwd=root, env=env, check=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--postgres", choices=["14", "15", "16", "17", "18"])
    parser.add_argument("--mysql", choices=["8.4", "9.7"])
    parser.add_argument("--toolchain", default="stable", choices=["stable", "nightly"])
    parser.add_argument("--full", action="store_true", help="run the full test feature matrix")
    args = parser.parse_args()
    if not (args.postgres or args.mysql):
        parser.error("select --postgres, --mysql, or both")
    if args.full and not (args.postgres and args.mysql):
        parser.error("--full requires both database versions")

    root = Path(__file__).resolve().parent.parent
    env = dict(os.environ)
    if args.postgres:
        env["POSTGRESQL_VERSION"] = args.postgres
    if args.mysql:
        env["MYSQL_VERSION"] = args.mysql
    base = json.loads(subprocess.check_output(
        ["docker", "compose", "-f", str(root / "docker-compose.yml"),
         "config", "--format", "json"], cwd=root, env=env, text=True))

    with tempfile.TemporaryDirectory(prefix="database-schema-tests.") as temporary:
        work = Path(temporary).resolve()
        output = work / "output"
        output.mkdir()
        binaries = work / "bin"
        binaries.mkdir()
        env["TMPDIR"] = str(output)
        env["PATH"] = str(binaries) + os.pathsep + env["PATH"]
        config = {"services": {}, "volumes": {}}
        selected = []
        for backend, version, service, data_path, commands in [
            ("postgres", args.postgres, "postgres-14", "/var/lib/postgresql", ["pg_dump", "psql"]),
            ("mysql", args.mysql, "mysql-8", "/var/lib/mysql", ["mysqldump", "mysql"]),
        ]:
            if version is None:
                continue
            selected.append((backend, service))
            settings = copy.deepcopy(base["services"][service])
            settings["volumes"] = [v for v in settings["volumes"] if v["target"] != data_path]
            settings["volumes"].extend([
                {"type": "volume", "source": backend + "-data", "target": data_path},
                {"type": "bind", "source": str(output), "target": str(output)},
            ])
            config["services"][service] = settings
            config["volumes"][backend + "-data"] = {}
            for command in commands:
                wrapper = binaries / command
                container_variable = "DATABASE_SCHEMA_TEST_" + backend.upper() + "_CONTAINER"
                forwarded = "-e PGPASSWORD -e PGSERVICEFILE " if backend == "postgres" else ""
                wrapper.write_text(
                    '#!/bin/sh\nexec docker exec -i ' + forwarded
                    + '"$' + container_variable + '" ' + command + ' "$@"\n')
                wrapper.chmod(0o755)
        compose_file = work / "compose.json"
        compose_file.write_text(json.dumps(config))
        project = "schema-tests-" + work.name.split(".", 1)[1]
        compose = ["docker", "compose", "--project-name", project, "-f", str(compose_file)]
        status = 0
        try:
            run(compose + ["up", "-d", "--wait", "--wait-timeout", "180"], root, env)
            for backend, service in selected:
                container = subprocess.check_output(
                    compose + ["ps", "--quiet", service], cwd=root, env=env, text=True).strip()
                env["DATABASE_SCHEMA_TEST_" + backend.upper() + "_CONTAINER"] = container
                client = "psql" if backend == "postgres" else "mysql"
                run(compose + ["exec", "-T", service, client, "--version"], root, env)
            features = FEATURES if args.full else [
                feature for backend, _ in selected
                for feature in [backend + ",diesel", backend + ",sqlx,runtime-tokio"]
            ]
            cargo = ["rustup", "run", args.toolchain, "cargo", "test"]
            for feature, no_defaults in [(f, False) for f in features] + (
                [(b + ",sqlx,runtime-tokio,macros,rustls", True)
                 for b in ["sqlite", "mysql", "postgres"]] if args.full else []
            ):
                run(cargo + (["--no-default-features"] if no_defaults else [])
                    + ["--features", feature, "--all-targets", "--verbose"], root, env)
                for backend, service in selected:
                    sql = "DROP TABLE IF EXISTS __diesel_schema_migrations,diesel_users,_sqlx_migrations,sqlx_users"
                    client_args = (["psql", "-U", "root", "postgres", "-c", sql]
                                   if backend == "postgres" else ["mysql", "-u", "root", "mysql", "-e", sql])
                    run(compose + ["exec", "-T", service] + client_args, root, env)
        except subprocess.CalledProcessError as error:
            status = error.returncode
        finally:
            cleanup = subprocess.run(compose + ["down", "--volumes"], cwd=root, env=env)
            if cleanup.returncode:
                status = status or cleanup.returncode
        return status


if __name__ == "__main__":
    raise SystemExit(main())

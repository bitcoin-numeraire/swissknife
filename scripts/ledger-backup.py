#!/usr/bin/env python3
"""Back up or restore one SwissKnife application database using age and native snapshots."""

import argparse
from contextlib import closing
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import sqlite3
import subprocess
import sys
import tarfile
import tempfile

CORE_TABLES = {"account", "asset", "wallet", "invoice", "payment", "seaql_migrations"}


def postgres_name(value):
    if not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_-]{0,62}", value):
        raise argparse.ArgumentTypeError("use a database name and pass connection settings through libpq")
    return value


def run(*args, **kwargs):
    return subprocess.run(args, check=True, **kwargs)


def check_sqlite(path):
    with closing(sqlite3.connect(path.resolve().as_uri() + "?mode=ro", uri=True)) as db:
        tables = {row[0] for row in db.execute("SELECT name FROM sqlite_schema WHERE type='table'")}
        if not CORE_TABLES <= tables:
            raise ValueError("snapshot is missing core SwissKnife tables")
        if db.execute("PRAGMA integrity_check").fetchall() != [("ok",)]:
            raise ValueError("SQLite integrity check failed")
        if db.execute("PRAGMA foreign_key_check").fetchone():
            raise ValueError("SQLite foreign key check failed")


def check_postgres_dump(snapshot):
    toc = run("pg_restore", "--list", snapshot, capture_output=True, text=True).stdout
    tables = set(re.findall(r"^\d+; \d+ \d+ TABLE public (\w+) ", toc, re.MULTILINE))
    if not CORE_TABLES <= tables:
        raise ValueError("dump is missing core SwissKnife tables in the public schema")


def digest(path):
    with path.open("rb") as stream:
        result = hashlib.sha256()
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            result.update(chunk)
        return result.hexdigest()


def publish(source, destination):
    """Publish a complete file without replacing an existing artifact, even on concurrent runs."""
    with tempfile.NamedTemporaryFile(dir=destination.parent) as partial:
        with source.open("rb") as data:
            shutil.copyfileobj(data, partial)
        partial.flush()
        os.fsync(partial.fileno())
        os.link(partial.name, destination)


def backup(args, work):
    if args.output.exists():
        raise ValueError("backup destination already exists")
    engine = "sqlite" if args.sqlite else "postgres"
    snapshot = work / "snapshot"
    started = datetime.now(timezone.utc).isoformat()
    if args.sqlite:
        source_uri = args.sqlite.resolve().as_uri() + "?mode=ro"
        with closing(sqlite3.connect(source_uri, uri=True)) as source, closing(sqlite3.connect(snapshot)) as target:
            source.backup(target, pages=256)
        check_sqlite(snapshot)
    else:
        run("pg_dump", "--no-password", "--format=custom", "--file", snapshot, "--dbname", args.postgres)
        check_postgres_dump(snapshot)

    manifest = {"format": 1, "engine": engine, "started_at": started, "sha256": digest(snapshot)}
    metadata = work / "manifest.json"
    metadata.write_text(json.dumps(manifest) + "\n", encoding="utf-8")
    encrypted = work / "backup.age"
    recipients = [arg for recipient in args.recipient for arg in ("--recipient", recipient)]
    with subprocess.Popen(["age", *recipients, "--output", encrypted], stdin=subprocess.PIPE) as age:
        with age.stdin as stream, tarfile.open(fileobj=stream, mode="w|") as archive:
            archive.add(metadata, arcname="manifest.json")
            archive.add(snapshot, arcname="snapshot")
        if age.wait():
            raise ValueError("age encryption failed")
    publish(encrypted, args.output)
    return manifest


def unpack(args, work):
    # Authenticate the entire ciphertext before opening the database or creating a restore target.
    archive_path = work / "archive.tar"
    run("age", "--decrypt", "--identity", args.identity, "--output", archive_path, args.input)
    with tarfile.open(archive_path, "r:") as archive:
        members = archive.getmembers()
        if sorted(m.name for m in members) != ["manifest.json", "snapshot"] or not all(m.isfile() for m in members):
            raise ValueError("unexpected ledger backup archive")
        metadata = archive.getmember("manifest.json")
        if metadata.size > 8192:
            raise ValueError("invalid backup manifest")
        with archive.extractfile(metadata) as stream:
            manifest = json.load(stream)
        if not isinstance(manifest, dict) or manifest.get("format") != 1:
            raise ValueError("unsupported backup format")
        engine = "sqlite" if args.sqlite else "postgres"
        if manifest.get("engine") != engine:
            raise ValueError("backup engine differs from the restore destination")
        snapshot = work / "snapshot"
        with archive.extractfile("snapshot") as source, snapshot.open("xb") as destination:
            shutil.copyfileobj(source, destination)
    if digest(snapshot) != manifest.get("sha256"):
        raise ValueError("snapshot checksum differs from the encrypted manifest")
    return manifest, snapshot


def restore(args, work):
    if args.sqlite and any(os.path.lexists(str(args.sqlite) + suffix) for suffix in ("", "-wal", "-shm", "-journal")):
        raise ValueError("SQLite destination or a journal already exists")
    manifest, snapshot = unpack(args, work)
    if args.sqlite:
        check_sqlite(snapshot)
        publish(snapshot, args.sqlite)
    else:
        check_postgres_dump(snapshot)
        # createdb fails if the name already exists. A failed restore leaves a new database for inspection.
        run("createdb", "--no-password", "--template=template0", "--", args.postgres)
        run("pg_restore", "--no-password", "--exit-on-error", "--single-transaction",
            "--no-owner", "--no-privileges", "--dbname", args.postgres, snapshot)
    return manifest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    for operation in (backup, restore):
        command = commands.add_parser(operation.__name__)
        db = command.add_mutually_exclusive_group(required=True)
        db.add_argument("--sqlite", type=Path, metavar="FILE")
        db.add_argument("--postgres", type=postgres_name, metavar="DATABASE")
        command.add_argument("--work-dir", type=Path, help="local private staging disk (defaults to system temporary storage)")
        if operation is backup:
            command.add_argument("--recipient", required=True, action="append", help="age public recipient, repeatable")
            command.add_argument("--output", required=True, type=Path)
        else:
            command.add_argument("--identity", required=True, type=Path, help="age identity file")
            command.add_argument("--input", required=True, type=Path)
        command.set_defaults(operation=operation)
    args = parser.parse_args()
    os.umask(0o077)
    with tempfile.TemporaryDirectory(dir=args.work_dir, prefix="swissknife-ledger-") as directory:
        result = args.operation(args, Path(directory))
    print(json.dumps(result))


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, sqlite3.Error, subprocess.CalledProcessError, tarfile.TarError) as error:
        sys.exit(str(error))

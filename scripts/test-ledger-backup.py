#!/usr/bin/env python3
"""Run on a host with age and age-keygen installed. All funds and credentials are synthetic."""
from concurrent.futures import ThreadPoolExecutor
from contextlib import closing
from pathlib import Path
import os
import uuid
import sqlite3
import subprocess
import sys
import tempfile
import unittest

TOOL = Path(__file__).with_name("ledger-backup.py")


class LedgerBackupTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.identity = self.root / "identity"
        subprocess.run(["age-keygen", "-o", self.identity], check=True, capture_output=True)
        self.recipient = subprocess.check_output(["age-keygen", "-y", self.identity], text=True).strip()
        self.artifact = self.root / "ledger.age"
        self.source = self.root / "live.sqlite"
        self.db = sqlite3.connect(self.source)
        self.addCleanup(self.db.close)
        self.db.execute("PRAGMA journal_mode=WAL")
        with self.db:
            for table in ["account", "asset", "wallet", "invoice", "payment", "seaql_migrations"]:
                self.db.execute(f"CREATE TABLE {table}(id INTEGER PRIMARY KEY, value TEXT)")
            self.db.execute("INSERT INTO wallet VALUES(1, '9007199254740993')")
            self.db.execute("INSERT INTO payment VALUES(1, 'pending')")

    def invoke(self, *args):
        return subprocess.run([sys.executable, TOOL, *args, "--work-dir", self.root], capture_output=True)

    def backup(self):
        return self.invoke("backup", "--sqlite", self.source, "--recipient", self.recipient, "--output", self.artifact)

    def restore(self, path):
        return self.invoke("restore", "--sqlite", path, "--identity", self.identity, "--input", self.artifact)

    def test_wal_snapshot_restores_exact_state_to_new_destination(self):
        self.assertEqual(self.backup().returncode, 0)
        with self.db:
            self.db.execute("UPDATE payment SET value='settled'")
        restored = self.root / "restored.sqlite"
        self.assertEqual(self.restore(restored).returncode, 0)
        with closing(sqlite3.connect(restored)) as db:
            self.assertEqual(db.execute("SELECT value FROM wallet").fetchone(), ("9007199254740993",))
            self.assertEqual(db.execute("SELECT value FROM payment").fetchone(), ("pending",))
        self.assertEqual(restored.stat().st_mode & 0o777, 0o600)
        self.assertNotEqual(self.restore(self.source).returncode, 0)
        self.assertEqual(self.db.execute("SELECT value FROM payment").fetchone(), ("settled",))

    def test_concurrent_backups_publish_once_and_damaged_ciphertext_leaves_no_database(self):
        with ThreadPoolExecutor(max_workers=2) as pool:
            results = list(pool.map(lambda _: self.backup().returncode, range(2)))
        self.assertEqual(sorted(results), [0, 1])
        data = self.artifact.read_bytes()
        self.artifact.write_bytes(data[:-10])
        target = self.root / "recovered.sqlite"
        self.assertNotEqual(self.restore(target).returncode, 0)
        self.assertFalse(target.exists())

    def test_unrelated_database_is_rejected_and_existing_sidecars_are_preserved(self):
        wrong = self.root / "other.sqlite"
        sqlite3.connect(wrong).close()
        result = self.invoke("backup", "--sqlite", wrong, "--recipient", self.recipient, "--output", self.artifact)
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(self.artifact.exists())
        self.assertEqual(self.backup().returncode, 0)
        target = self.root / "recovered.sqlite"
        journal = Path(str(target) + "-wal")
        journal.write_text("existing")
        self.assertNotEqual(self.restore(target).returncode, 0)
        self.assertFalse(target.exists())
        self.assertEqual(journal.read_text(), "existing")


@unittest.skipUnless(os.environ.get("SWISSKNIFE_BACKUP_TEST_POSTGRES") == "1", "requires a disposable PostgreSQL server")
class PostgresBackupTest(unittest.TestCase):
    def test_restore_preserves_ownership_reservations_and_payment_state(self):
        prefix = "swissknife_backup_test_" + uuid.uuid4().hex[:12]
        source, destination = prefix + "_src", prefix + "_dst"
        subprocess.run(["createdb", "--no-password", source], check=True)
        self.addCleanup(subprocess.run, ["dropdb", "--no-password", source], check=True)
        with tempfile.TemporaryDirectory() as directory:
            work = Path(directory)
            identity, artifact = work / "identity", work / "postgres.age"
            subprocess.run(["age-keygen", "-o", identity], check=True, capture_output=True)
            recipient = subprocess.check_output(["age-keygen", "-y", identity], text=True).strip()
            def query(db, sql):
                return subprocess.check_output(["psql", "-XAtqw", "--dbname", db, "-v", "ON_ERROR_STOP=1", "-c", sql], text=True)
            query(source, """
                CREATE TABLE account (id bigint PRIMARY KEY);
                CREATE TABLE asset (id bigint PRIMARY KEY);
                CREATE TABLE wallet (id bigint PRIMARY KEY, account_id bigint REFERENCES account, available bigint, reserved bigint);
                CREATE TABLE invoice (id bigint PRIMARY KEY);
                CREATE TABLE payment (id bigint PRIMARY KEY, wallet_id bigint REFERENCES wallet, status text);
                CREATE TABLE seaql_migrations (version text PRIMARY KEY);
                INSERT INTO account VALUES (1), (2);
                INSERT INTO wallet VALUES (3, 1, 9007199254740993, 21000), (4, 2, 0, 0);
                INSERT INTO payment VALUES (1, 3, 'Pending'), (2, 4, 'Settled');
            """)
            command = [sys.executable, TOOL]
            subprocess.run([*command, "backup", "--postgres", source, "--recipient", recipient, "--output", artifact, "--work-dir", work], check=True, capture_output=True)
            query(source, "UPDATE payment SET status='Settled' WHERE id=1")
            restore = [*command, "restore", "--postgres", destination, "--input", artifact, "--identity", identity, "--work-dir", work]
            subprocess.run(restore, check=True, capture_output=True)
            self.addCleanup(subprocess.run, ["dropdb", "--no-password", destination], check=True)
            self.assertEqual(query(destination, "SELECT * FROM wallet ORDER BY id").splitlines(), ["3|1|9007199254740993|21000", "4|2|0|0"])
            self.assertEqual(query(destination, "SELECT status FROM payment ORDER BY id").splitlines(), ["Pending", "Settled"])
            self.assertNotEqual(subprocess.run(restore, capture_output=True).returncode, 0)
            self.assertEqual(query(destination, "SELECT count(*) FROM payment").strip(), "2")


if __name__ == "__main__":
    unittest.main()

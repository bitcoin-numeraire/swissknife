# Database migrations

SwissKnife uses SeaORM migrations for both SQLite and PostgreSQL. Run migration
commands from the repository root so the Makefile supplies the correct crate
path.

```bash
make new-migration name=add-account-field
make run-migrations
make fresh-migrations
```

`make new-migration` generates the timestamped file and registers the normal
SeaORM skeleton. Implement the migration under `crates/migration/src/`, then add
its module and migrator entry to `crates/migration/src/lib.rs`.

After a schema change that affects SeaORM entities:

```bash
make up-postgres
DATABASE_URL=postgres://postgres:postgres@localhost:5432/numeraire make run-migrations
DATABASE_URL=postgres://postgres:postgres@localhost:5432/numeraire make generate-models
```

Review generated model changes carefully and keep both database engines covered
by migration or persistence tests.

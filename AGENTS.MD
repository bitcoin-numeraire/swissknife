# Development enviroment

These are the commands that can help you test, lint and handle the development environment.

- Run `make lint` to executes the linter (clippy)
- Run `make fmt` to execute the formatter (fmt)

# Data models and migrations

When changes are needed in the data models:

- You can generate new migrations using `make new-migration name=migration-name`. See other migrations under the `./migration` folder. Once a migration is added, you can work on it by modifying the file and then adding it in the `lib.rs` file.

The models are generated based on the schema and not the other way around.

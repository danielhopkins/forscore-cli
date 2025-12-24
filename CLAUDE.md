# forscore-cli

Rust CLI for managing forScore iPad app metadata via its SQLite database.

## Build

```
cargo build --release
```

## Structure

- `src/cli.rs` - clap command definitions
- `src/commands/` - command handlers
- `src/db.rs` - SQLite database access
- `src/models/` - data structures

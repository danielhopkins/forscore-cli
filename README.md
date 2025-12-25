# forscore-cli

A command-line tool for managing [forScore](https://forscore.co) metadata on macOS.

## Installation

```bash
cargo install --path .
```

Or build manually:

```bash
cargo build --release
# Binary at target/release/forscore
```

## Requirements

- macOS with forScore installed
- forScore must be syncing via iCloud (the CLI reads the local database)

## Usage

```bash
forscore <command> [options]
```

### Scores

```bash
forscore scores ls                      # List scores
forscore scores ls --library "Jazz"     # Filter by library
forscore scores ls --setlist "Gig"      # Filter by setlist
forscore scores ls --sort modified --desc
forscore scores search "Op 28"          # Search title or composer
forscore scores search --title "Prelude"
forscore scores search --composer "Bach"
forscore scores search --key "C Major"
forscore scores search --no-rating      # Find unrated scores
forscore scores show "Song Title"
forscore scores open "Song Title"       # Open in forScore
forscore scores edit "Song" --rating 5 --key "G Major"
```

### Setlists

```bash
forscore setlists ls
forscore setlists show "My Setlist"
forscore setlists create "New Setlist"
forscore setlists rename "Old Name" "New Name"
forscore setlists delete "Setlist"
forscore setlists add-score "Setlist" "Song Title"
forscore setlists remove-score "Setlist" "Song Title"
forscore setlists reorder "Setlist" "Song" --position 1
```

### Libraries

```bash
forscore libraries ls
forscore libraries show "Jazz"
forscore libraries add-score "Jazz" "Song"
forscore libraries remove-score "Jazz" "Song"
```

### Metadata

```bash
forscore composers ls
forscore composers ls --unused          # Find orphaned entries
forscore composers rename "JS Bach" "J.S. Bach"
forscore composers merge "Bach" "J.S. Bach"

forscore genres ls
forscore tags ls
```

### Bookmarks

```bash
forscore bookmarks ls "Score Name"
forscore bookmarks show 123
forscore bookmarks edit 123 --title "New Title"
forscore bookmarks delete 123
```

### Utilities

```bash
forscore info                           # Library statistics
forscore backup                         # Backup database
forscore backup -o backup.sqlite
forscore sync                           # iCloud sync status
forscore sync log                       # Recent sync activity
```

### Export/Import

```bash
forscore export csv -o scores.csv
forscore import csv scores.csv --dry-run
forscore import csv scores.csv
```

## Output Formats

Most commands support `--json` for machine-readable output:

```bash
forscore scores ls --json | jq '.[] | .title'
```

## License

MIT

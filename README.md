# Lightwave Relayer

A Rust application that relays proofs to a registry and performs health checks on [lightwave](https://github.com/timewave-computer/lightwave).

## Features

### Relayer Mode (default)
- Creates payloads and sends them to a registry
- Persists previous proof data in SQLite database (`relayer.db`)
- Continues from the last known proof if the server restarts
- Only sends new proofs when they differ from the previous one

### Health Check Mode
- Monitors light client proofs (Helios or Tendermint)
- Stores health check data in SQLite database (`health_check.db`)
- Tracks current height, current root, and timestamp
- Updates database when proof changes

## Database Schema

### Health Check Table
```sql
CREATE TABLE health_check (
    id INTEGER PRIMARY KEY,
    current_height INTEGER NOT NULL,
    current_root BLOB NOT NULL,
    timestamp TEXT NOT NULL
);
```

### Previous Proof Table
```sql
CREATE TABLE previous_proof (
    id INTEGER PRIMARY KEY,
    proof_data TEXT NOT NULL,
    timestamp TEXT NOT NULL
);
```

## Usage

### Run in Relayer Mode
```bash
cargo run --no-default-features --features relayer
```

### Run in Health Check Mode (default)
```bash
cargo run
```

## Database Files

- `relayer.db` - Created when running in relayer mode
- `health_check.db` - Created when running in health check mode

The database files are automatically created if they don't exist. Each mode maintains only the latest data (previous records are replaced when new data arrives).

## Dependencies

- `rusqlite` - SQLite database operations
- `chrono` - DateTime handling
- `serde` - Serialization/deserialization
- `tracing` - Logging
- `tempfile` - Testing utilities (dev dependency) 

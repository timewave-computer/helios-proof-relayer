use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthCheckData {
    pub current_height: u64,
    pub current_root: Vec<u8>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PreviousProof {
    pub proof_data: String,
    pub timestamp: DateTime<Utc>,
}

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        let db = Database {
            conn: Mutex::new(conn),
        };
        db.init_tables()?;
        Ok(db)
    }

    fn init_tables(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        // Create health_check table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS health_check (
                id INTEGER PRIMARY KEY,
                current_height INTEGER NOT NULL,
                current_root BLOB NOT NULL,
                timestamp TEXT NOT NULL
            )",
            [],
        )?;

        // Create previous_proof table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS previous_proof (
                id INTEGER PRIMARY KEY,
                proof_data TEXT NOT NULL,
                timestamp TEXT NOT NULL
            )",
            [],
        )?;

        Ok(())
    }

    pub fn update_health_check(&self, data: &HealthCheckData) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        // Delete existing record and insert new one (keeping only latest)
        conn.execute("DELETE FROM health_check", [])?;

        conn.execute(
            "INSERT INTO health_check (current_height, current_root, timestamp) VALUES (?1, ?2, ?3)",
            params![
                data.current_height,
                data.current_root,
                data.timestamp.to_rfc3339()
            ],
        )?;

        Ok(())
    }

    pub fn get_latest_health_check(&self) -> Result<Option<HealthCheckData>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT current_height, current_root, timestamp FROM health_check ORDER BY id DESC LIMIT 1"
        )?;

        let mut rows = stmt.query([])?;

        if let Some(row) = rows.next()? {
            let current_height: u64 = row.get(0)?;
            let current_root: Vec<u8> = row.get(1)?;
            let timestamp_str: String = row.get(2)?;
            let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)?.with_timezone(&Utc);

            Ok(Some(HealthCheckData {
                current_height,
                current_root,
                timestamp,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn update_previous_proof(&self, proof: &PreviousProof) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        // Delete existing record and insert new one (keeping only latest)
        conn.execute("DELETE FROM previous_proof", [])?;

        conn.execute(
            "INSERT INTO previous_proof (proof_data, timestamp) VALUES (?1, ?2)",
            params![proof.proof_data, proof.timestamp.to_rfc3339()],
        )?;

        Ok(())
    }

    pub fn get_previous_proof(&self) -> Result<Option<PreviousProof>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT proof_data, timestamp FROM previous_proof ORDER BY id DESC LIMIT 1")?;

        let mut rows = stmt.query([])?;

        if let Some(row) = rows.next()? {
            let proof_data: String = row.get(0)?;
            let timestamp_str: String = row.get(1)?;
            let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)?.with_timezone(&Utc);

            Ok(Some(PreviousProof {
                proof_data,
                timestamp,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn clear_all_tables(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        // Clear health_check table
        conn.execute("DELETE FROM health_check", [])?;

        // Clear previous_proof table
        conn.execute("DELETE FROM previous_proof", [])?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_database_operations() -> Result<()> {
        // Create a temporary database file
        let temp_file = NamedTempFile::new()?;
        let db_path = temp_file.path().to_str().unwrap();

        let db = Database::new(db_path)?;

        // Test health check data
        let health_data = HealthCheckData {
            current_height: 12345,
            current_root: vec![1, 2, 3, 4, 5],
            timestamp: Utc::now(),
        };

        db.update_health_check(&health_data)?;

        let retrieved_health = db.get_latest_health_check()?;
        assert!(retrieved_health.is_some());
        let retrieved_health = retrieved_health.unwrap();
        assert_eq!(retrieved_health.current_height, 12345);
        assert_eq!(retrieved_health.current_root, vec![1, 2, 3, 4, 5]);

        // Test previous proof
        let proof_data = PreviousProof {
            proof_data: "test_proof_data".to_string(),
            timestamp: Utc::now(),
        };

        db.update_previous_proof(&proof_data)?;

        let retrieved_proof = db.get_previous_proof()?;
        assert!(retrieved_proof.is_some());
        let retrieved_proof = retrieved_proof.unwrap();
        assert_eq!(retrieved_proof.proof_data, "test_proof_data");

        Ok(())
    }
}

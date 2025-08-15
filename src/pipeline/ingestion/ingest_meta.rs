use rusqlite::{params, Connection};
use std::path::Path;

pub struct IngestMeta {
    conn: Connection,
}

impl IngestMeta {
    pub fn open_at_root<P: AsRef<Path>>(data_root: P) -> anyhow::Result<Self> {
        let db_path = data_root.as_ref().join("ingest_log").join("meta.db");
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(db_path)?;
        conn.execute_batch(
            r#"
            PRAGMA journal_mode=WAL;
            CREATE TABLE IF NOT EXISTS dedupe_index (
                idempotency_key TEXT PRIMARY KEY,
                envelope_id     TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS consumer_offsets (
                consumer     TEXT PRIMARY KEY,
                byte_offset  INTEGER NOT NULL,
                envelope_id  TEXT
            );
            CREATE TABLE IF NOT EXISTS fetch_cadence (
                source_id        TEXT PRIMARY KEY,
                last_fetched_at  INTEGER NOT NULL
            );
            "#,
        )?;
        Ok(Self { conn })
    }

    // Dedupe index
    pub fn get_envelope_by_idk(&self, idk: &str) -> anyhow::Result<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT envelope_id FROM dedupe_index WHERE idempotency_key = ?1")?;
        let mut rows = stmt.query(params![idk])?;
        if let Some(row) = rows.next()? {
            let eid: String = row.get(0)?;
            Ok(Some(eid))
        } else {
            Ok(None)
        }
    }

    pub fn put_dedupe_mapping(&self, idk: &str, envelope_id: &str) -> anyhow::Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO dedupe_index (idempotency_key, envelope_id) VALUES (?1, ?2)",
            params![idk, envelope_id],
        )?;
        Ok(())
    }

    // Consumer offsets
    pub fn get_offset(&self, consumer: &str) -> anyhow::Result<(u64, Option<String>)> {
        let mut stmt = self
            .conn
            .prepare("SELECT byte_offset, envelope_id FROM consumer_offsets WHERE consumer = ?1")?;
        let mut rows = stmt.query(params![consumer])?;
        if let Some(row) = rows.next()? {
            let off: u64 = row.get::<_, i64>(0)? as u64;
            let eid: Option<String> = row.get(1).ok();
            Ok((off, eid))
        } else {
            Ok((0, None))
        }
    }

    pub fn set_offset(
        &self,
        consumer: &str,
        byte_offset: u64,
        envelope_id: Option<&str>,
    ) -> anyhow::Result<()> {
        self.conn.execute(
            "INSERT INTO consumer_offsets (consumer, byte_offset, envelope_id) VALUES (?1, ?2, ?3)
             ON CONFLICT(consumer) DO UPDATE SET byte_offset=excluded.byte_offset, envelope_id=excluded.envelope_id",
            params![consumer, byte_offset as i64, envelope_id],
        )?;
        Ok(())
    }

    // Simple cadence tracking (e.g., twice a day per source)
    pub fn get_last_fetched_at(&self, source_id: &str) -> anyhow::Result<Option<i64>> {
        let mut stmt = self
            .conn
            .prepare("SELECT last_fetched_at FROM fetch_cadence WHERE source_id = ?1")?;
        let mut rows = stmt.query(params![source_id])?;
        if let Some(row) = rows.next()? {
            let ts: i64 = row.get(0)?;
            Ok(Some(ts))
        } else {
            Ok(None)
        }
    }

    pub fn set_last_fetched_at(&self, source_id: &str, ts: i64) -> anyhow::Result<()> {
        self.conn.execute(
            "INSERT INTO fetch_cadence (source_id, last_fetched_at) VALUES (?1, ?2)
             ON CONFLICT(source_id) DO UPDATE SET last_fetched_at=excluded.last_fetched_at",
            params![source_id, ts],
        )?;
        Ok(())
    }
}

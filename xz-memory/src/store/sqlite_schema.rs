/// SQL DDL for the SQLite-backed memory system.
pub const DDL: &[&str] = &[
    // Messages table
    "CREATE TABLE IF NOT EXISTS messages (
        id TEXT PRIMARY KEY,
        session_id TEXT NOT NULL,
        user_id TEXT NOT NULL,
        role TEXT NOT NULL,
        content TEXT NOT NULL,
        token_count INTEGER DEFAULT 0,
        created_at INTEGER NOT NULL,
        seq INTEGER NOT NULL
    )",
    "CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id, seq)",
    "CREATE INDEX IF NOT EXISTS idx_messages_user ON messages(user_id, created_at)",
    // Session summaries table
    "CREATE TABLE IF NOT EXISTS session_summaries (
        session_id TEXT PRIMARY KEY,
        user_id TEXT NOT NULL,
        summary TEXT NOT NULL,
        key_points_json TEXT DEFAULT '[]',
        token_count INTEGER DEFAULT 0,
        message_count INTEGER DEFAULT 0,
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL
    )",
    "CREATE INDEX IF NOT EXISTS idx_summaries_user ON session_summaries(user_id, updated_at)",
    // Facts table
    "CREATE TABLE IF NOT EXISTS facts (
        id TEXT PRIMARY KEY,
        user_id TEXT NOT NULL,
        category TEXT NOT NULL DEFAULT 'Custom',
        subject TEXT NOT NULL,
        predicate TEXT NOT NULL,
        object TEXT NOT NULL,
        confidence REAL DEFAULT 0.5,
        source_session TEXT,
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL,
        version INTEGER DEFAULT 1
    )",
    "CREATE INDEX IF NOT EXISTS idx_facts_user ON facts(user_id)",
    "CREATE INDEX IF NOT EXISTS idx_facts_subject_predicate ON facts(user_id, subject, predicate)",
    "CREATE INDEX IF NOT EXISTS idx_facts_category ON facts(user_id, category)",
    // FTS5 virtual table for facts
    "CREATE VIRTUAL TABLE IF NOT EXISTS facts_fts USING fts5(
        subject, predicate, object,
        content='facts',
        content_rowid='rowid'
    )",
    // Vectors table (for vector-memory feature)
    "CREATE TABLE IF NOT EXISTS vectors (
        id TEXT PRIMARY KEY,
        user_id TEXT NOT NULL,
        content TEXT,
        embedding BLOB,
        metadata_json TEXT DEFAULT '{}',
        created_at INTEGER NOT NULL,
        dimension INTEGER DEFAULT 0,
        expires_at INTEGER,
        channel TEXT
    )",
    "CREATE INDEX IF NOT EXISTS idx_vectors_user ON vectors(user_id)",
];

/// Triggers to keep FTS5 index in sync with facts table.
pub const FTS_TRIGGERS: &[&str] = &[
    "CREATE TRIGGER IF NOT EXISTS facts_fts_insert AFTER INSERT ON facts BEGIN
        INSERT INTO facts_fts(rowid, subject, predicate, object)
        VALUES (new.rowid, new.subject, new.predicate, new.object);
    END",
    "CREATE TRIGGER IF NOT EXISTS facts_fts_delete AFTER DELETE ON facts BEGIN
        INSERT INTO facts_fts(facts_fts, rowid, subject, predicate, object)
        VALUES ('delete', old.rowid, old.subject, old.predicate, old.object);
    END",
    "CREATE TRIGGER IF NOT EXISTS facts_fts_update AFTER UPDATE ON facts BEGIN
        INSERT INTO facts_fts(facts_fts, rowid, subject, predicate, object)
        VALUES ('delete', old.rowid, old.subject, old.predicate, old.object);
        INSERT INTO facts_fts(rowid, subject, predicate, object)
        VALUES (new.rowid, new.subject, new.predicate, new.object);
    END",
];

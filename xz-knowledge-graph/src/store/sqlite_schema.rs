pub const DDL: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS entities (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        entity_type TEXT NOT NULL,
        attributes_json TEXT NOT NULL DEFAULT '{}',
        description TEXT,
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL,
        version INTEGER NOT NULL DEFAULT 1,
        source TEXT,
        tags_json TEXT NOT NULL DEFAULT '[]',
        aliases_json TEXT NOT NULL DEFAULT '[]'
    )",
    "CREATE INDEX IF NOT EXISTS idx_entities_name ON entities(name)",
    "CREATE INDEX IF NOT EXISTS idx_entities_type ON entities(entity_type)",
    "CREATE INDEX IF NOT EXISTS idx_entities_created ON entities(created_at)",
    "CREATE INDEX IF NOT EXISTS idx_entities_source ON entities(source)",
    "CREATE TABLE IF NOT EXISTS relations (
        id TEXT PRIMARY KEY,
        source_id TEXT NOT NULL,
        target_id TEXT NOT NULL,
        relation_type TEXT NOT NULL,
        properties_json TEXT NOT NULL DEFAULT '{}',
        confidence REAL NOT NULL DEFAULT 0.5,
        provenance_json TEXT,
        valid_from INTEGER,
        valid_to INTEGER,
        created_at INTEGER NOT NULL,
        weight REAL
    )",
    "CREATE INDEX IF NOT EXISTS idx_relations_source ON relations(source_id)",
    "CREATE INDEX IF NOT EXISTS idx_relations_target ON relations(target_id)",
    "CREATE INDEX IF NOT EXISTS idx_relations_type ON relations(relation_type)",
    "CREATE INDEX IF NOT EXISTS idx_relations_confidence ON relations(confidence)",
    "CREATE INDEX IF NOT EXISTS idx_relations_valid ON relations(valid_from, valid_to)",
    "CREATE VIRTUAL TABLE IF NOT EXISTS entities_fts USING fts5(
        name, aliases, description, content='entities', content_rowid='rowid'
    )",
];

pub const FTS_TRIGGERS: &[&str] = &[
    "CREATE TRIGGER IF NOT EXISTS entities_fts_insert AFTER INSERT ON entities BEGIN
        INSERT INTO entities_fts(rowid, name, aliases, description)
        VALUES (new.rowid, new.name, new.aliases_json, new.description);
    END",
    "CREATE TRIGGER IF NOT EXISTS entities_fts_delete AFTER DELETE ON entities BEGIN
        INSERT INTO entities_fts(entities_fts, rowid, name, aliases, description)
        VALUES ('delete', old.rowid, old.name, old.aliases_json, old.description);
    END",
    "CREATE TRIGGER IF NOT EXISTS entities_fts_update AFTER UPDATE ON entities BEGIN
        INSERT INTO entities_fts(entities_fts, rowid, name, aliases, description)
        VALUES ('delete', old.rowid, old.name, old.aliases_json, old.description);
        INSERT INTO entities_fts(rowid, name, aliases, description)
        VALUES (new.rowid, new.name, new.aliases_json, new.description);
    END",
];

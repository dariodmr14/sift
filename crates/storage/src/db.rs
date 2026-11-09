//! Conexión a SQLite y migraciones.
//!
//! Las migraciones usan `PRAGMA user_version`, un entero que SQLite guarda en el propio
//! fichero. Cada migración es un bloque SQL; al abrir, se aplican en orden las que falten
//! (cada una en su transacción) y se sube el número. Nunca se edita una migración ya
//! publicada: los cambios van en una nueva al final de la lista.

use std::path::Path;

use rusqlite::Connection;

use crate::StorageError;

/// Migraciones en orden. El índice + 1 es la versión que deja la base de datos.
const MIGRATIONS: &[&str] = &[
    // v1: Espacios, sus alias y acciones, y comandos aprobados.
    r#"
    CREATE TABLE spaces (
        id          INTEGER PRIMARY KEY,
        position    INTEGER NOT NULL,
        name        TEXT    NOT NULL,
        icon        TEXT    NOT NULL,              -- JSON de `Icon`
        created_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        updated_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
    );

    CREATE TABLE space_aliases (
        space_id  INTEGER NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
        position  INTEGER NOT NULL,
        alias     TEXT    NOT NULL,
        PRIMARY KEY (space_id, position)
    );

    CREATE TABLE space_actions (
        space_id  INTEGER NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
        position  INTEGER NOT NULL,
        kind      TEXT    NOT NULL,                -- copia del tipo, para consultas
        action    TEXT    NOT NULL,                -- JSON de `Action`
        PRIMARY KEY (space_id, position)
    );

    -- Un comando aprobado es exactamente (programa, argumentos, carpeta): si cambia
    -- cualquiera de los tres, es otro comando y se vuelve a pedir confirmación.
    CREATE TABLE command_approvals (
        program      TEXT NOT NULL,
        args         TEXT NOT NULL,                -- JSON array
        cwd          TEXT NOT NULL,                -- '' si no hay carpeta
        approved_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        PRIMARY KEY (program, args, cwd)
    );
    "#,
];

pub struct Database {
    pub(crate) conn: Connection,
}

impl Database {
    /// Abre (o crea) la base de datos en `path` y la pone al día.
    pub fn open(path: &Path) -> Result<Database, StorageError> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| StorageError::Io(e.to_string()))?;
        }
        let conn = Connection::open(path)?;
        // WAL: las lecturas no esperan a las escrituras y un corte de luz no corrompe.
        conn.pragma_update(None, "journal_mode", "WAL")?;
        Self::prepare(conn)
    }

    /// Base de datos en memoria, para los tests.
    pub fn open_in_memory() -> Result<Database, StorageError> {
        Self::prepare(Connection::open_in_memory()?)
    }

    fn prepare(conn: Connection) -> Result<Database, StorageError> {
        // En SQLite las claves foráneas (y su ON DELETE CASCADE) van desactivadas por
        // defecto y hay que encenderlas en cada conexión.
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let mut db = Database { conn };
        db.migrate()?;
        Ok(db)
    }

    pub fn schema_version(&self) -> Result<u32, StorageError> {
        Ok(self
            .conn
            .pragma_query_value(None, "user_version", |r| r.get(0))?)
    }

    fn migrate(&mut self) -> Result<(), StorageError> {
        let current = self.schema_version()? as usize;
        if current > MIGRATIONS.len() {
            // Una versión más nueva de Sift creó esta base de datos: no la tocamos.
            return Err(StorageError::NewerSchema(current as u32));
        }
        for (i, sql) in MIGRATIONS.iter().enumerate().skip(current) {
            let tx = self.conn.transaction()?;
            tx.execute_batch(sql)?;
            tx.pragma_update(None, "user_version", (i + 1) as u32)?;
            tx.commit()?;
            tracing::info!(version = i + 1, "migración de la base de datos aplicada");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_database_gets_the_latest_schema() {
        let db = Database::open_in_memory().unwrap();
        assert_eq!(db.schema_version().unwrap() as usize, MIGRATIONS.len());
    }

    #[test]
    fn reopening_does_not_reapply_migrations() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sift.db");
        Database::open(&path).unwrap();
        // Si volviera a aplicar la v1, fallaría con "table spaces already exists".
        let db = Database::open(&path).unwrap();
        assert_eq!(db.schema_version().unwrap() as usize, MIGRATIONS.len());
    }

    #[test]
    fn refuses_a_database_from_a_newer_version() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sift.db");
        {
            let conn = Connection::open(&path).unwrap();
            conn.pragma_update(None, "user_version", 999).unwrap();
        }
        assert!(matches!(
            Database::open(&path),
            Err(StorageError::NewerSchema(999))
        ));
    }

    #[test]
    fn foreign_keys_are_enforced() {
        let db = Database::open_in_memory().unwrap();
        let orphan = db.conn.execute(
            "INSERT INTO space_actions (space_id, position, kind, action) VALUES (42, 0, 'x', '{}')",
            [],
        );
        assert!(orphan.is_err());
    }
}

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
    // v2: indexado de archivos (Fase 2, ver ADR 0006). Los fragmentos (`chunks`) son la
    // unidad de búsqueda y serán la de los embeddings en la Fase 4 (clave: chunks.id).
    r#"
    CREATE TABLE roots (
        id               INTEGER PRIMARY KEY,
        path             TEXT NOT NULL UNIQUE,
        added_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        last_indexed_at  TEXT
    );

    CREATE TABLE ignore_patterns (
        position  INTEGER NOT NULL PRIMARY KEY,
        pattern   TEXT    NOT NULL
    );
    -- Patrones por defecto: carpetas de dependencias, compilación y temporales.
    INSERT INTO ignore_patterns (position, pattern) VALUES
        (0, '.git'), (1, 'node_modules'), (2, 'target'), (3, 'dist'), (4, 'build'),
        (5, '.venv'), (6, 'venv'), (7, '__pycache__'), (8, '.idea'), (9, '.vscode'),
        (10, '$RECYCLE.BIN'), (11, '*.tmp'), (12, '~$*'), (13, 'Thumbs.db'), (14, 'desktop.ini');

    CREATE TABLE files (
        id            INTEGER PRIMARY KEY,
        root_id       INTEGER NOT NULL REFERENCES roots(id) ON DELETE CASCADE,
        path          TEXT    NOT NULL UNIQUE,
        name          TEXT    NOT NULL,
        ext           TEXT    NOT NULL,             -- en minúsculas, sin punto ('' si no hay)
        size          INTEGER NOT NULL,
        mtime         INTEGER NOT NULL,             -- milisegundos desde 1970
        content_hash  TEXT,                         -- blake3 del contenido (si se extrajo)
        indexed_at    TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        error         TEXT                          -- por qué no se pudo leer (si pasó)
    );
    CREATE INDEX files_root ON files(root_id);

    CREATE TABLE chunks (
        id         INTEGER PRIMARY KEY,
        file_id    INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
        ordinal    INTEGER NOT NULL,                -- posición dentro del archivo
        start      INTEGER NOT NULL,                -- desplazamiento en caracteres
        text       TEXT    NOT NULL,
        UNIQUE (file_id, ordinal)
    );

    -- Qué abres y cuándo (frecuencia de uso, Fase 5).
    CREATE TABLE opens (
        id         INTEGER PRIMARY KEY,
        file_id    INTEGER REFERENCES files(id) ON DELETE CASCADE,
        space_id   INTEGER REFERENCES spaces(id) ON DELETE CASCADE,
        opened_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
        CHECK ((file_id IS NULL) <> (space_id IS NULL))
    );
    CREATE INDEX opens_file ON opens(file_id);
    "#,
    // v3: por qué no se pudo leer un archivo, en categorías («formato no compatible»,
    // «demasiado grande»…), y más patrones a ignorar por defecto: carpetas de compilación,
    // entornos, cachés y carpetas de trabajo de editores (Eclipse, Visual Studio…).
    r#"
    ALTER TABLE files ADD COLUMN error_kind TEXT;   -- código de `UnreadReason`
    UPDATE files SET error_kind = CASE
        WHEN error LIKE 'parece un archivo binario%' THEN 'unsupported'
        WHEN error LIKE 'demasiado grande%'          THEN 'tooLarge'
        ELSE 'other' END
    WHERE error IS NOT NULL;
    CREATE INDEX files_error_kind ON files(error_kind) WHERE error_kind IS NOT NULL;

    -- Solo se añaden los que falten, detrás de los que ya hubiera (tuyos o por defecto).
    WITH new(pattern, n) AS (VALUES
            ('bin', 1),
            ('obj', 2),
            ('out', 3),
            ('.next', 4),
            ('.nuxt', 5),
            ('.gradle', 6),
            ('*.egg-info', 7),
            ('site-packages', 8),
            ('.tox', 9),
            ('bower_components', 10),
            ('Pods', 11),
            ('.cache', 12),
            ('.pytest_cache', 13),
            ('.mypy_cache', 14),
            ('.ruff_cache', 15),
            ('.ipynb_checkpoints', 16),
            ('.metadata', 17),
            ('.settings', 18),
            ('.vs', 19),
            ('.history', 20),
            ('.svn', 21),
            ('.hg', 22))
    INSERT INTO ignore_patterns (position, pattern)
    SELECT (SELECT COALESCE(MAX(position), -1) FROM ignore_patterns) + n, pattern
    FROM new
    WHERE pattern NOT IN (SELECT pattern FROM ignore_patterns);
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

    /// Devuelve al disco el espacio de lo borrado (SQLite no encoge el fichero solo).
    /// Tarda en bases de datos grandes: solo tras borrados masivos.
    pub fn compact(&self) -> Result<(), StorageError> {
        self.conn.execute_batch("VACUUM")?;
        Ok(())
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
    fn upgrades_a_v1_database_keeping_its_data() {
        // Una base de datos real de la Fase 1.5: solo la v1 aplicada y un Espacio dentro.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sift.db");
        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(MIGRATIONS[0]).unwrap();
            conn.pragma_update(None, "user_version", 1).unwrap();
            conn.execute(
                "INSERT INTO spaces (position, name, icon) VALUES (0, 'Uni', '{\"kind\":\"auto\"}')",
                [],
            )
            .unwrap();
        }
        let db = Database::open(&path).unwrap();
        assert_eq!(db.schema_version().unwrap() as usize, MIGRATIONS.len());
        assert_eq!(db.list_spaces().unwrap()[0].draft.name, "Uni");
        assert!(db.list_roots().unwrap().is_empty());
        assert!(!db.ignore_patterns().unwrap().is_empty());
    }

    #[test]
    fn v3_classifies_old_errors_and_adds_patterns_without_touching_yours() {
        // Una base de datos de la Fase 2 (v2), con errores sin clasificar y un patrón
        // propio; uno de los nuevos por defecto («bin») ya lo había puesto el usuario.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sift.db");
        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(MIGRATIONS[0]).unwrap();
            conn.execute_batch(MIGRATIONS[1]).unwrap();
            conn.pragma_update(None, "user_version", 2).unwrap();
            conn.execute_batch(
                "INSERT INTO roots (id, path) VALUES (1, 'C:\\Uni');
                 INSERT INTO files (root_id, path, name, ext, size, mtime, error) VALUES
                    (1, 'a', 'a', 'txt', 1, 1, 'parece un archivo binario'),
                    (1, 'b', 'b', 'txt', 1, 1, 'demasiado grande para leer su contenido (9 bytes)'),
                    (1, 'c', 'c', 'txt', 1, 1, 'no se pudo leer: acceso denegado'),
                    (1, 'd', 'd', 'txt', 1, 1, NULL);
                 INSERT INTO ignore_patterns (position, pattern) VALUES (15, 'Antiguo'), (16, 'bin');",
            )
            .unwrap();
        }
        let db = Database::open(&path).unwrap();
        let kinds: Vec<Option<String>> = db
            .conn
            .prepare("SELECT error_kind FROM files ORDER BY path")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(
            kinds,
            [
                Some("unsupported".into()),
                Some("tooLarge".into()),
                Some("other".into()),
                None
            ]
        );
        let patterns = db.ignore_patterns().unwrap();
        assert!(patterns.contains(&"Antiguo".to_string()));
        assert!(patterns.contains(&".metadata".to_string()));
        assert_eq!(patterns.iter().filter(|p| *p == "bin").count(), 1);
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

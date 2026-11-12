//! Repositorio de Espacios: crear, leer, actualizar, duplicar, borrar, reordenar,
//! importar y aprobar comandos. Todo lo que escribe va en una transacción, así que un
//! fallo a mitad nunca deja un Espacio con la mitad de sus acciones.

use rusqlite::{OptionalExtension, Transaction, params};
use sift_spaces::{Action, CommandSpec, Icon, Space, SpaceDraft, validate};

use crate::{Database, StorageError};

impl Database {
    pub fn list_spaces(&self) -> Result<Vec<Space>, StorageError> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, icon FROM spaces ORDER BY position, id")?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        rows.into_iter()
            .map(|(id, name, icon)| self.hydrate(id, name, &icon))
            .collect()
    }

    pub fn get_space(&self, id: i64) -> Result<Space, StorageError> {
        let row = self
            .conn
            .query_row("SELECT name, icon FROM spaces WHERE id = ?1", [id], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })
            .optional()?
            .ok_or(StorageError::NotFound(id))?;
        self.hydrate(id, row.0, &row.1)
    }

    /// Crea un Espacio al final de la lista.
    pub fn create_space(&mut self, draft: SpaceDraft) -> Result<Space, StorageError> {
        let draft = validate(draft)?;
        let tx = self.conn.transaction()?;
        let id = insert(&tx, &draft, next_position(&tx)?)?;
        tx.commit()?;
        Ok(Space { id, draft })
    }

    pub fn update_space(&mut self, id: i64, draft: SpaceDraft) -> Result<Space, StorageError> {
        let draft = validate(draft)?;
        let tx = self.conn.transaction()?;
        let changed = tx.execute(
            "UPDATE spaces SET name = ?1, icon = ?2,
                    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?3",
            params![draft.name, to_json(&draft.icon)?, id],
        )?;
        if changed == 0 {
            return Err(StorageError::NotFound(id));
        }
        // Alias y acciones se reescriben enteros: son listas cortas y así el orden
        // guardado es siempre exactamente el de la UI.
        tx.execute("DELETE FROM space_aliases WHERE space_id = ?1", [id])?;
        tx.execute("DELETE FROM space_actions WHERE space_id = ?1", [id])?;
        insert_children(&tx, id, &draft)?;
        tx.commit()?;
        Ok(Space { id, draft })
    }

    /// Copia un Espacio justo debajo del original, con " (copia)" en el nombre.
    pub fn duplicate_space(&mut self, id: i64) -> Result<Space, StorageError> {
        let mut draft = self.get_space(id)?.draft;
        draft.name = copy_name(&draft.name);
        let tx = self.conn.transaction()?;
        let position: i64 =
            tx.query_row("SELECT position FROM spaces WHERE id = ?1", [id], |r| {
                r.get(0)
            })?;
        tx.execute(
            "UPDATE spaces SET position = position + 1 WHERE position > ?1",
            [position],
        )?;
        let new_id = insert(&tx, &draft, position + 1)?;
        tx.commit()?;
        Ok(Space { id: new_id, draft })
    }

    pub fn delete_space(&mut self, id: i64) -> Result<(), StorageError> {
        let tx = self.conn.transaction()?;
        // ON DELETE CASCADE borra también sus alias y acciones.
        if tx.execute("DELETE FROM spaces WHERE id = ?1", [id])? == 0 {
            return Err(StorageError::NotFound(id));
        }
        compact_positions(&tx)?;
        tx.commit()?;
        Ok(())
    }

    /// Guarda un orden nuevo. `ids` tiene que contener exactamente los Espacios que hay.
    pub fn reorder_spaces(&mut self, ids: &[i64]) -> Result<(), StorageError> {
        let tx = self.conn.transaction()?;
        let mut current: Vec<i64> = tx
            .prepare("SELECT id FROM spaces")?
            .query_map([], |r| r.get(0))?
            .collect::<Result<_, _>>()?;
        let mut wanted = ids.to_vec();
        current.sort_unstable();
        wanted.sort_unstable();
        if current != wanted {
            return Err(StorageError::ReorderMismatch);
        }
        for (position, id) in ids.iter().enumerate() {
            tx.execute(
                "UPDATE spaces SET position = ?1 WHERE id = ?2",
                params![position as i64, id],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Añade al final los Espacios importados (todos o ninguno). Devuelve cuántos.
    pub fn import_spaces(&mut self, drafts: Vec<SpaceDraft>) -> Result<usize, StorageError> {
        let drafts = drafts
            .into_iter()
            .map(validate)
            .collect::<Result<Vec<_>, _>>()?;
        let tx = self.conn.transaction()?;
        let first = next_position(&tx)?;
        for (position, draft) in (first..).zip(&drafts) {
            insert(&tx, draft, position)?;
        }
        tx.commit()?;
        Ok(drafts.len())
    }

    /// Apunta que has abierto un Espacio (para la frecencia).
    pub fn record_space_open(&mut self, id: i64) -> Result<(), StorageError> {
        self.conn
            .execute("INSERT INTO opens (space_id) VALUES (?1)", [id])?;
        Ok(())
    }

    /// Las aperturas de Espacios de los últimos `max_days` días: (Espacio, hace cuántos
    /// días), para la frecencia.
    pub fn space_open_ages(&self, max_days: f64) -> Result<Vec<(i64, f64)>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT space_id, julianday('now') - julianday(opened_at) AS age
             FROM opens WHERE space_id IS NOT NULL AND age <= ?1",
        )?;
        let rows = stmt
            .query_map([max_days], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    pub fn is_command_approved(&self, cmd: &CommandSpec) -> Result<bool, StorageError> {
        let (program, args, cwd) = approval_key(cmd)?;
        Ok(self
            .conn
            .query_row(
                "SELECT 1 FROM command_approvals WHERE program = ?1 AND args = ?2 AND cwd = ?3",
                params![program, args, cwd],
                |_| Ok(()),
            )
            .optional()?
            .is_some())
    }

    pub fn approve_command(&mut self, cmd: &CommandSpec) -> Result<(), StorageError> {
        let (program, args, cwd) = approval_key(cmd)?;
        self.conn.execute(
            "INSERT OR IGNORE INTO command_approvals (program, args, cwd) VALUES (?1, ?2, ?3)",
            params![program, args, cwd],
        )?;
        Ok(())
    }

    fn hydrate(&self, id: i64, name: String, icon: &str) -> Result<Space, StorageError> {
        let aliases = self
            .conn
            .prepare("SELECT alias FROM space_aliases WHERE space_id = ?1 ORDER BY position")?
            .query_map([id], |r| r.get(0))?
            .collect::<Result<Vec<String>, _>>()?;
        let actions = self
            .conn
            .prepare("SELECT action FROM space_actions WHERE space_id = ?1 ORDER BY position")?
            .query_map([id], |r| r.get::<_, String>(0))?
            .map(|json| Ok(serde_json::from_str::<Action>(&json?)?))
            .collect::<Result<Vec<_>, StorageError>>()?;
        Ok(Space {
            id,
            draft: SpaceDraft {
                name,
                aliases,
                icon: serde_json::from_str::<Icon>(icon)?,
                actions,
            },
        })
    }
}

fn insert(tx: &Transaction, draft: &SpaceDraft, position: i64) -> Result<i64, StorageError> {
    tx.execute(
        "INSERT INTO spaces (position, name, icon) VALUES (?1, ?2, ?3)",
        params![position, draft.name, to_json(&draft.icon)?],
    )?;
    let id = tx.last_insert_rowid();
    insert_children(tx, id, draft)?;
    Ok(id)
}

fn insert_children(tx: &Transaction, id: i64, draft: &SpaceDraft) -> Result<(), StorageError> {
    for (i, alias) in draft.aliases.iter().enumerate() {
        tx.execute(
            "INSERT INTO space_aliases (space_id, position, alias) VALUES (?1, ?2, ?3)",
            params![id, i as i64, alias],
        )?;
    }
    for (i, action) in draft.actions.iter().enumerate() {
        tx.execute(
            "INSERT INTO space_actions (space_id, position, kind, action) VALUES (?1, ?2, ?3, ?4)",
            params![id, i as i64, kind(action), to_json(action)?],
        )?;
    }
    Ok(())
}

fn next_position(tx: &Transaction) -> Result<i64, StorageError> {
    Ok(tx.query_row(
        "SELECT COALESCE(MAX(position) + 1, 0) FROM spaces",
        [],
        |r| r.get(0),
    )?)
}

/// Deja las posiciones seguidas (0, 1, 2…) tras borrar.
fn compact_positions(tx: &Transaction) -> Result<(), StorageError> {
    let ids: Vec<i64> = tx
        .prepare("SELECT id FROM spaces ORDER BY position, id")?
        .query_map([], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    for (position, id) in ids.iter().enumerate() {
        tx.execute(
            "UPDATE spaces SET position = ?1 WHERE id = ?2",
            params![position as i64, id],
        )?;
    }
    Ok(())
}

fn kind(action: &Action) -> &'static str {
    match action {
        Action::OpenFolder { .. } => "openFolder",
        Action::OpenFile { .. } => "openFile",
        Action::OpenUrl { .. } => "openUrl",
        Action::OpenApp { .. } => "openApp",
        Action::RunCommand(_) => "runCommand",
        Action::Wait(_) => "wait",
    }
}

fn approval_key(cmd: &CommandSpec) -> Result<(String, String, String), StorageError> {
    let cwd = cmd
        .cwd
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    Ok((cmd.program.clone(), serde_json::to_string(&cmd.args)?, cwd))
}

fn to_json<T: serde::Serialize>(value: &T) -> Result<String, StorageError> {
    Ok(serde_json::to_string(value)?)
}

fn copy_name(name: &str) -> String {
    const SUFFIX: &str = " (copia)";
    let max = sift_spaces::validate::MAX_NAME - SUFFIX.chars().count();
    let base: String = name.chars().take(max).collect();
    format!("{base}{SUFFIX}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn draft(name: &str) -> SpaceDraft {
        SpaceDraft {
            name: name.into(),
            aliases: vec!["bd".into(), "bases".into()],
            icon: Icon::Emoji {
                value: "🗄️".into()
            },
            actions: vec![
                Action::OpenUrl {
                    url: "https://campus.es".into(),
                },
                Action::RunCommand(CommandSpec {
                    program: "docker".into(),
                    args: vec!["compose".into(), "up".into()],
                    cwd: Some(PathBuf::from(r"C:\Uni\BD")),
                }),
            ],
        }
    }

    fn names(db: &Database) -> Vec<String> {
        db.list_spaces()
            .unwrap()
            .into_iter()
            .map(|s| s.draft.name)
            .collect()
    }

    #[test]
    fn create_and_read_back_everything_in_order() {
        let mut db = Database::open_in_memory().unwrap();
        let created = db.create_space(draft("Bases de datos")).unwrap();
        let read = db.get_space(created.id).unwrap();
        assert_eq!(read, created);
        assert_eq!(read.draft.aliases, vec!["bd", "bases"]);
        assert_eq!(read.draft.actions.len(), 2);
    }

    #[test]
    fn space_opens_are_recorded_and_go_with_the_space() {
        let mut db = Database::open_in_memory().unwrap();
        let a = db.create_space(draft("A")).unwrap().id;
        let b = db.create_space(draft("B")).unwrap().id;
        db.record_space_open(a).unwrap();
        db.record_space_open(a).unwrap();
        db.record_space_open(b).unwrap();
        let ages = db.space_open_ages(30.0).unwrap();
        assert_eq!(ages.iter().filter(|(id, _)| *id == a).count(), 2);
        assert!(ages.iter().all(|(_, age)| (0.0..0.01).contains(age)));
        // Al borrar el Espacio se van sus aperturas.
        db.delete_space(a).unwrap();
        let ages = db.space_open_ages(30.0).unwrap();
        assert_eq!(ages.len(), 1);
        assert_eq!(ages[0].0, b);
    }

    #[test]
    fn invalid_spaces_are_not_saved() {
        let mut db = Database::open_in_memory().unwrap();
        assert!(matches!(
            db.create_space(draft("   ")),
            Err(StorageError::Invalid(_))
        ));
        assert!(db.list_spaces().unwrap().is_empty());
    }

    #[test]
    fn update_replaces_aliases_and_actions() {
        let mut db = Database::open_in_memory().unwrap();
        let s = db.create_space(draft("A")).unwrap();
        let mut d = s.draft.clone();
        d.name = "B".into();
        d.aliases = vec!["solo".into()];
        d.actions.reverse();
        db.update_space(s.id, d.clone()).unwrap();
        assert_eq!(db.get_space(s.id).unwrap().draft, d);
        assert!(matches!(
            db.update_space(999, d),
            Err(StorageError::NotFound(999))
        ));
    }

    #[test]
    fn duplicate_goes_right_below_the_original() {
        let mut db = Database::open_in_memory().unwrap();
        let a = db.create_space(draft("A")).unwrap();
        db.create_space(draft("B")).unwrap();
        let copy = db.duplicate_space(a.id).unwrap();
        assert_eq!(names(&db), vec!["A", "A (copia)", "B"]);
        assert_eq!(copy.draft.actions, a.draft.actions);
        assert_ne!(copy.id, a.id);
    }

    #[test]
    fn delete_cascades_and_compacts() {
        let mut db = Database::open_in_memory().unwrap();
        let a = db.create_space(draft("A")).unwrap();
        db.create_space(draft("B")).unwrap();
        db.delete_space(a.id).unwrap();
        assert_eq!(names(&db), vec!["B"]);
        let orphans: i64 = db
            .conn
            .query_row(
                "SELECT COUNT(*) FROM space_actions WHERE space_id = ?1",
                [a.id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(orphans, 0);
        // Tras borrar, el siguiente va al final sin huecos.
        db.create_space(draft("C")).unwrap();
        assert_eq!(names(&db), vec!["B", "C"]);
    }

    #[test]
    fn reorder_requires_the_exact_set_of_ids() {
        let mut db = Database::open_in_memory().unwrap();
        let a = db.create_space(draft("A")).unwrap().id;
        let b = db.create_space(draft("B")).unwrap().id;
        let c = db.create_space(draft("C")).unwrap().id;
        db.reorder_spaces(&[c, a, b]).unwrap();
        assert_eq!(names(&db), vec!["C", "A", "B"]);
        assert!(matches!(
            db.reorder_spaces(&[c, a]),
            Err(StorageError::ReorderMismatch)
        ));
        assert!(matches!(
            db.reorder_spaces(&[c, a, 99]),
            Err(StorageError::ReorderMismatch)
        ));
    }

    #[test]
    fn import_is_all_or_nothing() {
        let mut db = Database::open_in_memory().unwrap();
        db.create_space(draft("Existente")).unwrap();
        assert_eq!(db.import_spaces(vec![draft("X"), draft("Y")]).unwrap(), 2);
        assert_eq!(names(&db), vec!["Existente", "X", "Y"]);
        assert!(db.import_spaces(vec![draft("Z"), draft("")]).is_err());
        assert_eq!(names(&db).len(), 3); // ni siquiera "Z"
    }

    #[test]
    fn approvals_are_tied_to_the_exact_command() {
        let mut db = Database::open_in_memory().unwrap();
        let up = CommandSpec {
            program: "docker".into(),
            args: vec!["compose".into(), "up".into()],
            cwd: None,
        };
        assert!(!db.is_command_approved(&up).unwrap());
        db.approve_command(&up).unwrap();
        db.approve_command(&up).unwrap(); // aprobar dos veces no falla
        assert!(db.is_command_approved(&up).unwrap());

        let mut other_args = up.clone();
        other_args.args.push("-d".into());
        let mut other_dir = up.clone();
        other_dir.cwd = Some(PathBuf::from(r"C:\otra"));
        assert!(!db.is_command_approved(&other_args).unwrap());
        assert!(!db.is_command_approved(&other_dir).unwrap());
    }

    #[test]
    fn copy_name_respects_the_length_limit() {
        let long = "x".repeat(80);
        assert!(copy_name(&long).chars().count() <= sift_spaces::validate::MAX_NAME);
        assert_eq!(copy_name("BD"), "BD (copia)");
    }
}

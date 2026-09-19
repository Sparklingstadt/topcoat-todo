use rusqlite::{Connection, params};
use std::{path::PathBuf, time::Duration};

#[derive(Clone)]
pub struct Database {
    path: PathBuf,
}

pub struct Todo {
    pub id: i64,
    pub title: String,
    pub completed: bool,
}

impl Database {
    pub async fn open(path: impl Into<PathBuf>) -> topcoat::Result<Self> {
        let db = Self { path: path.into() };
        if let Some(parent) = db.path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        db.run(|connection| {
            connection.execute_batch(
                "PRAGMA journal_mode = WAL;
                 CREATE TABLE IF NOT EXISTS todos (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    title TEXT NOT NULL CHECK(length(trim(title)) BETWEEN 1 AND 200),
                    completed INTEGER NOT NULL DEFAULT 0 CHECK(completed IN (0, 1))
                 );",
            )?;
            Ok(())
        })
        .await?;
        Ok(db)
    }

    // Keep SQLite work off Tokio's async worker threads. Each request gets a
    // connection; SQLite serializes writes and busy_timeout handles contention.
    async fn run<T, F>(&self, operation: F) -> topcoat::Result<T>
    where
        T: Send + 'static,
        F: FnOnce(&Connection) -> rusqlite::Result<T> + Send + 'static,
    {
        let path = self.path.clone();
        Ok(tokio::task::spawn_blocking(move || {
            let connection = Connection::open(path)?;
            connection.busy_timeout(Duration::from_secs(5))?;
            operation(&connection)
        })
        .await??)
    }

    pub async fn list(&self) -> topcoat::Result<Vec<Todo>> {
        self.run(|connection| {
            let mut statement = connection.prepare(
                "SELECT id, title, completed FROM todos ORDER BY completed ASC, id DESC",
            )?;
            statement
                .query_map([], |row| {
                    Ok(Todo {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        completed: row.get(2)?,
                    })
                })?
                .collect()
        })
        .await
    }

    pub async fn add(&self, title: String) -> topcoat::Result<()> {
        self.run(move |connection| {
            connection.execute("INSERT INTO todos(title) VALUES (?1)", params![title])?;
            Ok(())
        })
        .await
    }

    pub async fn set_completed(&self, id: i64, completed: bool) -> topcoat::Result<bool> {
        self.run(move |connection| {
            Ok(connection.execute(
                "UPDATE todos SET completed = ?1 WHERE id = ?2",
                params![completed, id],
            )? == 1)
        })
        .await
    }

    pub async fn delete(&self, id: i64) -> topcoat::Result<bool> {
        self.run(move |connection| {
            Ok(connection.execute("DELETE FROM todos WHERE id = ?1", params![id])? == 1)
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn database() -> (tempfile::TempDir, Database) {
        let directory = tempfile::tempdir().unwrap();
        let db = Database::open(directory.path().join("nested/todos.sqlite3"))
            .await
            .unwrap();
        (directory, db)
    }

    #[tokio::test]
    async fn new_database_creates_parent_directories_and_starts_empty() {
        let (directory, db) = database().await;
        assert!(directory.path().join("nested/todos.sqlite3").is_file());
        assert!(db.list().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn incomplete_items_come_first_and_newer_items_come_first() {
        let (_directory, db) = database().await;
        db.add("最初のTodo".into()).await.unwrap();
        let first = db.list().await.unwrap()[0].id;
        db.add("次のTodo".into()).await.unwrap();
        db.add("最新のTodo".into()).await.unwrap();
        let newest = db.list().await.unwrap()[0].id;
        db.set_completed(newest, true).await.unwrap();
        let todos = db.list().await.unwrap();
        assert_eq!(
            todos
                .iter()
                .map(|todo| todo.title.as_str())
                .collect::<Vec<_>>(),
            ["次のTodo", "最初のTodo", "最新のTodo"]
        );
        assert_eq!(todos[1].id, first);
        assert!(!todos[0].completed);
        assert!(todos[2].completed);
    }

    #[tokio::test]
    async fn completion_is_idempotent_and_reversible() {
        let (_directory, db) = database().await;
        db.add("テスト".into()).await.unwrap();
        let id = db.list().await.unwrap()[0].id;
        for _ in 0..2 {
            assert!(db.set_completed(id, true).await.unwrap());
            assert!(db.list().await.unwrap()[0].completed);
        }
        assert!(db.set_completed(id, false).await.unwrap());
        assert!(!db.list().await.unwrap()[0].completed);
        assert!(!db.set_completed(i64::MAX, true).await.unwrap());
    }

    #[tokio::test]
    async fn deletion_only_removes_the_requested_item() {
        let (_directory, db) = database().await;
        db.add("残す".into()).await.unwrap();
        db.add("削除する".into()).await.unwrap();
        let id = db.list().await.unwrap()[0].id;
        assert!(db.delete(id).await.unwrap());
        assert!(!db.delete(id).await.unwrap());
        let remaining = db.list().await.unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].title, "残す");
    }

    #[tokio::test]
    async fn reopening_keeps_titles_ids_and_completed_state() {
        let (directory, db) = database().await;
        db.add("再起動後も残る🍅".into()).await.unwrap();
        let id = db.list().await.unwrap()[0].id;
        db.set_completed(id, true).await.unwrap();
        drop(db);
        let reopened = Database::open(directory.path().join("nested/todos.sqlite3"))
            .await
            .unwrap();
        let todos = reopened.list().await.unwrap();
        assert_eq!(todos.len(), 1);
        assert_eq!(todos[0].id, id);
        assert_eq!(todos[0].title, "再起動後も残る🍅");
        assert!(todos[0].completed);
    }

    #[tokio::test]
    async fn concurrent_additions_do_not_lose_rows_or_reuse_ids() {
        let (_directory, db) = database().await;
        let mut tasks = tokio::task::JoinSet::new();
        for index in 0..16 {
            let db = db.clone();
            tasks.spawn(async move { db.add(format!("Todo {index}")).await });
        }
        while let Some(result) = tasks.join_next().await {
            result.unwrap().unwrap();
        }
        let todos = db.list().await.unwrap();
        assert_eq!(todos.len(), 16);
        assert_eq!(
            todos
                .iter()
                .map(|todo| todo.id)
                .collect::<std::collections::HashSet<_>>()
                .len(),
            16
        );
        assert_eq!(
            todos
                .iter()
                .map(|todo| &todo.title)
                .collect::<std::collections::HashSet<_>>()
                .len(),
            16
        );
    }

    #[tokio::test]
    async fn sql_like_text_is_saved_as_plain_data() {
        let (_directory, db) = database().await;
        let title = "'); DROP TABLE todos; -- <script>日本語</script>";
        db.add(title.into()).await.unwrap();
        db.add("テーブルは残っている".into()).await.unwrap();
        let todos = db.list().await.unwrap();
        assert_eq!(todos.len(), 2);
        assert_eq!(todos[1].title, title);
    }

    #[tokio::test]
    async fn database_constraints_reject_invalid_lengths_without_changing_rows() {
        let (_directory, db) = database().await;
        for title in ["".into(), "   ".into(), "あ".repeat(201)] {
            assert!(db.add(title).await.is_err());
        }
        assert!(db.list().await.unwrap().is_empty());
        db.add("あ".repeat(200)).await.unwrap();
        assert_eq!(db.list().await.unwrap()[0].title.chars().count(), 200);
    }
}

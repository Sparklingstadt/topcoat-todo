use crate::model::Priority;
use rusqlite::{Connection, TransactionBehavior, params};
use std::{path::PathBuf, time::Duration};

#[derive(Clone)]
pub struct Database {
    path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct Todo {
    pub id: i64,
    pub title: String,
    pub completed: bool,
    pub due_date: Option<String>,
    pub priority: Priority,
}

impl Database {
    pub async fn open(path: impl Into<PathBuf>) -> topcoat::Result<Self> {
        let db = Self { path: path.into() };
        if let Some(parent) = db.path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        db.run(|connection| {
            connection.execute_batch("PRAGMA journal_mode = WAL;")?;
            let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            transaction.execute_batch(
                "CREATE TABLE IF NOT EXISTS todos (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    title TEXT NOT NULL CHECK(length(trim(title)) BETWEEN 1 AND 200),
                    completed INTEGER NOT NULL DEFAULT 0 CHECK(completed IN (0, 1))
                 );",
            )?;
            let columns: Vec<String> = {
                let mut statement = transaction.prepare("PRAGMA table_info(todos)")?;
                statement.query_map([], |row| row.get(1))?.collect::<rusqlite::Result<_>>()?
            };
            if !columns.iter().any(|name| name == "due_date") {
                transaction.execute_batch("ALTER TABLE todos ADD COLUMN due_date TEXT;")?;
            }
            if !columns.iter().any(|name| name == "priority") {
                transaction.execute_batch("ALTER TABLE todos ADD COLUMN priority INTEGER NOT NULL DEFAULT 1 CHECK(priority BETWEEN 0 AND 2);")?;
            }
            transaction.commit()?;
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
        F: FnOnce(&mut Connection) -> rusqlite::Result<T> + Send + 'static,
    {
        let path = self.path.clone();
        Ok(tokio::task::spawn_blocking(move || {
            let mut connection = Connection::open(path)?;
            connection.busy_timeout(Duration::from_secs(5))?;
            operation(&mut connection)
        })
        .await??)
    }

    pub async fn list(&self) -> topcoat::Result<Vec<Todo>> {
        self.run(|connection| {
            let mut statement = connection.prepare(
                "SELECT id, title, completed, due_date, priority FROM todos ORDER BY completed ASC, priority DESC, due_date IS NULL, due_date ASC, id DESC",
            )?;
            statement
                .query_map([], |row| {
                    Ok(Todo {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        completed: row.get(2)?,
                        due_date: row.get(3)?,
                        priority: Priority::from_number(row.get(4)?),
                    })
                })?
                .collect()
        })
        .await
    }

    #[cfg(test)]
    pub async fn add(&self, title: String) -> topcoat::Result<()> {
        self.add_details(title, None, Priority::Normal).await
    }

    pub async fn add_details(
        &self,
        title: String,
        due_date: Option<String>,
        priority: Priority,
    ) -> topcoat::Result<()> {
        self.run(move |connection| {
            connection.execute(
                "INSERT INTO todos(title, due_date, priority) VALUES (?1, ?2, ?3)",
                params![title, due_date, priority as i64],
            )?;
            Ok(())
        })
        .await
    }

    pub async fn update(
        &self,
        id: i64,
        title: String,
        due_date: Option<String>,
        priority: Priority,
    ) -> topcoat::Result<bool> {
        self.run(move |connection| {
            Ok(connection.execute(
                "UPDATE todos SET title = ?1, due_date = ?2, priority = ?3 WHERE id = ?4",
                params![title, due_date, priority as i64, id],
            )? == 1)
        })
        .await
    }

    // The confirmation form sends a snapshot of IDs. Newly completed items or
    // items restored to active after confirmation must not be deleted.
    pub async fn delete_completed(&self, ids: Vec<i64>) -> topcoat::Result<usize> {
        self.run(move |connection| {
            let transaction =
                connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let mut count = 0;
            for id in ids {
                count += transaction.execute(
                    "DELETE FROM todos WHERE id = ?1 AND completed = 1",
                    params![id],
                )?;
            }
            transaction.commit()?;
            Ok(count)
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
    #[tokio::test]
    async fn migration_preserves_v010_rows_and_is_safe_to_repeat() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("old.sqlite3");
        let connection = Connection::open(&path).unwrap();
        connection.execute_batch("CREATE TABLE todos (id INTEGER PRIMARY KEY AUTOINCREMENT, title TEXT NOT NULL CHECK(length(trim(title)) BETWEEN 1 AND 200), completed INTEGER NOT NULL DEFAULT 0 CHECK(completed IN (0, 1))); INSERT INTO todos (id,title,completed) VALUES (41,'以前のTodo',1);").unwrap();
        drop(connection);
        for _ in 0..2 {
            let db = Database::open(&path).await.unwrap();
            let todo = db.list().await.unwrap().remove(0);
            assert_eq!(todo.id, 41);
            assert_eq!(todo.title, "以前のTodo");
            assert!(todo.completed);
            assert_eq!(todo.due_date, None);
            assert_eq!(todo.priority, Priority::Normal);
        }
        let db = Database::open(&path).await.unwrap();
        db.add("新しいTodo".into()).await.unwrap();
        assert_eq!(db.list().await.unwrap()[0].id, 42);
    }

    #[tokio::test]
    async fn edit_persists_fields_preserves_completion_and_can_clear_date() {
        let (directory, db) = database().await;
        db.add("編集前".into()).await.unwrap();
        let id = db.list().await.unwrap()[0].id;
        db.set_completed(id, true).await.unwrap();
        assert!(
            db.update(
                id,
                "編集後".into(),
                Some("2028-02-29".into()),
                Priority::High
            )
            .await
            .unwrap()
        );
        drop(db);
        let db = Database::open(directory.path().join("nested/todos.sqlite3"))
            .await
            .unwrap();
        let todo = db.list().await.unwrap().remove(0);
        assert_eq!(todo.title, "編集後");
        assert!(todo.completed);
        assert_eq!(todo.priority, Priority::High);
        assert_eq!(todo.due_date.as_deref(), Some("2028-02-29"));
        assert!(
            db.update(id, "期限なし".into(), None, Priority::Low)
                .await
                .unwrap()
        );
        let todo = db.list().await.unwrap().remove(0);
        assert!(todo.completed);
        assert_eq!(todo.due_date, None);
        assert_eq!(todo.priority, Priority::Low);
        assert!(
            !db.update(i64::MAX, "存在しない".into(), None, Priority::Normal)
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn priority_then_date_order_is_stable_and_completion_stays_last() {
        let (_directory, db) = database().await;
        for (title, date, priority) in [
            ("低・早い", Some("2026-01-01"), Priority::Low),
            ("高・期限なし", None, Priority::High),
            ("高・遅い", Some("2026-12-01"), Priority::High),
            ("高・早い", Some("2026-02-01"), Priority::High),
            ("通常", None, Priority::Normal),
        ] {
            db.add_details(title.into(), date.map(str::to_owned), priority)
                .await
                .unwrap();
        }
        let todos = db.list().await.unwrap();
        assert_eq!(
            todos
                .iter()
                .map(|todo| todo.title.as_str())
                .collect::<Vec<_>>(),
            ["高・早い", "高・遅い", "高・期限なし", "通常", "低・早い"]
        );
        db.set_completed(todos[0].id, true).await.unwrap();
        assert_eq!(db.list().await.unwrap().last().unwrap().title, "高・早い");
    }

    #[tokio::test]
    async fn bulk_delete_only_removes_selected_items_still_completed() {
        let (_directory, db) = database().await;
        for title in ["対象", "未完了に戻す", "後で完了", "未完了"] {
            db.add(title.into()).await.unwrap();
        }
        let todos = db.list().await.unwrap();
        let id = |title: &str| todos.iter().find(|todo| todo.title == title).unwrap().id;
        db.set_completed(id("対象"), true).await.unwrap();
        db.set_completed(id("未完了に戻す"), true).await.unwrap();
        let snapshot = vec![id("対象"), id("未完了に戻す"), id("未完了"), i64::MAX];
        db.set_completed(id("未完了に戻す"), false).await.unwrap();
        db.set_completed(id("後で完了"), true).await.unwrap();
        assert_eq!(db.delete_completed(snapshot.clone()).await.unwrap(), 1);
        assert_eq!(db.delete_completed(snapshot).await.unwrap(), 0);
        assert_eq!(db.delete_completed(vec![]).await.unwrap(), 0);
        assert_eq!(db.list().await.unwrap().len(), 3);
    }

    #[tokio::test]
    async fn rejected_edit_keeps_every_original_field() {
        let (_directory, db) = database().await;
        db.add_details("保存済み".into(), Some("2028-02-29".into()), Priority::High)
            .await
            .unwrap();
        let id = db.list().await.unwrap()[0].id;
        db.set_completed(id, true).await.unwrap();
        for invalid_title in [String::new(), "あ".repeat(201)] {
            assert!(
                db.update(id, invalid_title, None, Priority::Low)
                    .await
                    .is_err()
            );
            let todo = db.list().await.unwrap().remove(0);
            assert_eq!(todo.id, id);
            assert_eq!(todo.title, "保存済み");
            assert!(todo.completed);
            assert_eq!(todo.due_date.as_deref(), Some("2028-02-29"));
            assert_eq!(todo.priority, Priority::High);
        }
    }

    #[tokio::test]
    async fn bulk_failure_rolls_back_prior_deletions_in_the_same_request() {
        let (_directory, db) = database().await;
        for title in ["最初", "途中で失敗", "対象外"] {
            db.add(title.into()).await.unwrap();
        }
        let todos = db.list().await.unwrap();
        let first = todos.iter().find(|todo| todo.title == "最初").unwrap().id;
        let failing = todos
            .iter()
            .find(|todo| todo.title == "途中で失敗")
            .unwrap()
            .id;
        for id in [first, failing] {
            db.set_completed(id, true).await.unwrap();
        }
        db.run(|connection| {
            connection.execute_batch(
                "CREATE TRIGGER test_reject_delete BEFORE DELETE ON todos
             WHEN OLD.title = '途中で失敗'
             BEGIN SELECT RAISE(ABORT, 'injected test failure'); END;",
            )
        })
        .await
        .unwrap();
        assert!(db.delete_completed(vec![first, failing]).await.is_err());
        let remaining = db.list().await.unwrap();
        assert_eq!(remaining.len(), 3);
        assert!(
            remaining
                .iter()
                .any(|todo| todo.id == first && todo.completed)
        );
        assert!(
            remaining
                .iter()
                .any(|todo| todo.id == failing && todo.completed)
        );
        db.run(|connection| connection.execute_batch("DROP TRIGGER test_reject_delete;"))
            .await
            .unwrap();
        assert_eq!(db.delete_completed(vec![first, failing]).await.unwrap(), 2);
        assert_eq!(db.list().await.unwrap()[0].title, "対象外");
    }

    #[tokio::test]
    async fn concurrent_edit_and_completion_preserve_both_changes() {
        let (_directory, db) = database().await;
        db.add("変更前".into()).await.unwrap();
        let id = db.list().await.unwrap()[0].id;
        let (edited, completed) = tokio::join!(
            db.update(
                id,
                "変更後".into(),
                Some("2028-03-01".into()),
                Priority::High
            ),
            db.set_completed(id, true)
        );
        assert!(edited.unwrap());
        assert!(completed.unwrap());
        let todo = db.list().await.unwrap().remove(0);
        assert_eq!(todo.title, "変更後");
        assert!(todo.completed);
        assert_eq!(todo.due_date.as_deref(), Some("2028-03-01"));
        assert_eq!(todo.priority, Priority::High);
    }

    #[tokio::test]
    async fn tied_dates_use_newest_id_without_losing_duplicate_titles() {
        let (_directory, db) = database().await;
        let mut ids = Vec::new();
        for _ in 0..3 {
            db.add_details("同じTodo".into(), Some("2028-03-01".into()), Priority::High)
                .await
                .unwrap();
            ids.push(db.list().await.unwrap()[0].id);
        }
        assert_eq!(
            db.list()
                .await
                .unwrap()
                .iter()
                .map(|todo| todo.id)
                .collect::<Vec<_>>(),
            vec![ids[2], ids[1], ids[0]]
        );
        db.update(
            ids[0],
            "最初だけ変更".into(),
            Some("2028-03-01".into()),
            Priority::High,
        )
        .await
        .unwrap();
        assert_eq!(
            db.list()
                .await
                .unwrap()
                .iter()
                .map(|todo| todo.id)
                .collect::<Vec<_>>(),
            vec![ids[2], ids[1], ids[0]]
        );
    }
}

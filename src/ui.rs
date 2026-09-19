use topcoat::{
    Result,
    view::{View, component, view},
};

use crate::{db::Todo, routes::Filter};

#[component]
pub async fn document(
    todos: Vec<Todo>,
    filter: Filter,
    error: Option<&'static str>,
) -> Result<impl View> {
    let total = todos.len();
    let completed = todos.iter().filter(|todo| todo.completed).count();
    let active = total - completed;
    let percent = (completed * 100).checked_div(total).unwrap_or(0);
    let visible: Vec<_> = todos
        .into_iter()
        .filter(|todo| match filter {
            Filter::All => true,
            Filter::Active => !todo.completed,
            Filter::Completed => todo.completed,
        })
        .collect();

    Ok(view! {
        <!DOCTYPE html>
        <html lang="ja">
            <head>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                <meta name="theme-color" content="#f6f5f1">
                <meta name="description" content="ひとつずつ、やることを整理するシンプルなTodo。">
                <title>"ひとつずつ。 | Topcoat Todo"</title>
                <link rel="stylesheet" href="/style.css">
                <link rel="icon" href="data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 40 40'%3E%3Crect width='40' height='40' rx='12' fill='%23344339'/%3E%3Cpath d='m11 20 6 6 12-13' fill='none' stroke='%23fff' stroke-width='3'/%3E%3C/svg%3E">
            </head>
            <body>
                <a class="skip-link" href="#main">"Todoへ移動"</a>
                <div class="app-shell">
                    <header class="brand-bar">
                        <a class="brand" href="/" aria-label="ひとつずつ。 ホーム">
                            <span class="brand-icon" aria-hidden="true">icon(kind: "check")</span>
                            <span>"ひとつずつ。"</span>
                        </a>
                        <span class="private-note"><span class="status-dot"></span>"マイリスト"</span>
                    </header>
                    <main id="main">
                        <section class="overview" aria-labelledby="page-title">
                            <div>
                                <p class="eyebrow">"MY EVERYDAY"</p>
                                <h1 id="page-title">"やることリスト"</h1>
                                <p class="summary">
                                    if total == 0 {
                                        "頭の中のやることを、ここに。"
                                    } else if active == 0 {
                                        "すべて完了。おつかれさまでした！"
                                    } else {
                                        "あと " <strong>(active)</strong> " 件。ひとつずつ進めましょう。"
                                    }
                                </p>
                            </div>
                            <div class="progress-card" role="group" aria-label=(format!("{}件中{}件完了", total, completed))>
                                <span class="progress-label">"完了したTodo"</span>
                                <div class="progress-count"><strong>(completed)</strong><span>"/ " (total)</span></div>
                                <div class="progress-track"><span style=(format!("width: {percent}%"))></span></div>
                            </div>
                        </section>
                        <section class="todo-panel" aria-label="Todoの管理">
                            <form class="add-form" action="/todos/add" method="post">
                                <input type="hidden" name="filter" value=(filter.value())>
                                <label class="sr-only" for="todo-title">"新しいTodo"</label>
                                <span class="input-plus" aria-hidden="true">"＋"</span>
                                <input id="todo-title" name="title" type="text" required=(true) maxlength="200" autocomplete="off" placeholder="次にやることを入力…">
                                <button class="add-button" type="submit">"追加" icon(kind: "arrow")</button>
                            </form>
                            if let Some(message) = error {
                                <p class="error-message" role="alert">(message)</p>
                            }
                            <div class="list-toolbar">
                                <nav class="filters" aria-label="Todoの絞り込み">
                                    filter_link(label: "すべて", count: total, value: Filter::All, current: filter)
                                    filter_link(label: "未完了", count: active, value: Filter::Active, current: filter)
                                    filter_link(label: "完了", count: completed, value: Filter::Completed, current: filter)
                                </nav>
                                <span class="list-count">(visible.len()) " 件"</span>
                            </div>
                            if visible.is_empty() {
                                empty_state(filter: filter, total: total)
                            } else {
                                <ul class="todo-list">
                                    for todo in visible {
                                        todo_row(todo: todo, filter: filter)
                                    }
                                </ul>
                            }
                            <div class="panel-footer">
                                <span>icon(kind: "check") "チェックして完了"</span>
                                <span>"自動保存"</span>
                            </div>
                        </section>
                    </main>
                    <footer class="app-footer"><span>"小さな一歩を、積み重ねよう。"</span><span>"TOPCOAT TODO"</span></footer>
                </div>
            </body>
        </html>
    })
}

#[component]
async fn filter_link(
    label: &'static str,
    count: usize,
    value: Filter,
    current: Filter,
) -> Result<impl View> {
    Ok(view! {
        <a href=(format!("/?filter={}", value.value())) if value == current { aria-current="page" }>
            (label)<span>(count)</span>
        </a>
    })
}

#[component]
async fn todo_row(todo: Todo, filter: Filter) -> Result<impl View> {
    Ok(view! {
        <li class=(if todo.completed { "todo-row is-complete" } else { "todo-row" }) data-todo-id=(todo.id)>
            <form class="complete-form" action="/todos/complete" method="post">
                <input type="hidden" name="id" value=(todo.id)>
                <input type="hidden" name="completed" value=((!todo.completed).to_string())>
                <input type="hidden" name="filter" value=(filter.value())>
                <button class="complete-button" type="submit" aria-pressed=(todo.completed.to_string()) aria-label=(format!("{}を{}", todo.title, if todo.completed { "未完了に戻す" } else { "完了にする" }))>
                    <span class="checkbox" aria-hidden="true">if todo.completed { icon(kind: "check") }</span>
                    <span class="todo-title">(&todo.title)</span>
                </button>
            </form>
            <form class="delete-form" action="/todos/delete" method="post">
                <input type="hidden" name="id" value=(todo.id)>
                <input type="hidden" name="filter" value=(filter.value())>
                <button class="delete-button" type="submit" aria-label=(format!("{}を削除", todo.title)) title="削除">icon(kind: "trash")</button>
            </form>
        </li>
    })
}

#[component]
async fn empty_state(filter: Filter, total: usize) -> Result<impl View> {
    let (title, hint) = match filter {
        Filter::Completed => (
            "完了したTodoはまだありません",
            "終わったTodoにチェックを付けると、ここに表示されます。",
        ),
        Filter::Active if total > 0 => (
            "すべてのTodoが完了しました",
            "ひと区切り。少し休憩しましょう。",
        ),
        _ => (
            "最初のTodoを追加しましょう",
            "小さなことでも大丈夫。上の欄から追加できます。",
        ),
    };
    Ok(view! {
        <div class="empty-state">
            <div class="empty-illustration" aria-hidden="true">
                <div class="paper"><span class="paper-line"></span><span class="paper-line"></span><span class="paper-line short"></span></div>
                <span class="paper-check">icon(kind: "check")</span>
                <span class="spark spark-one">"＋"</span><span class="spark spark-two">"·"</span>
            </div>
            <h2>(title)</h2><p>(hint)</p>
        </div>
    })
}

#[component]
async fn icon(kind: &'static str) -> Result<impl View> {
    Ok(view! {
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            match kind {
                "trash" => { <path d="M4 7h16M9 7V4h6v3M6 7l1 13h10l1-13M10 11v5M14 11v5"></path> }
                "arrow" => { <path d="M5 12h14m-5-5 5 5-5 5"></path> }
                _ => { <path d="m5 12 4 4L19 6"></path> }
            }
        </svg>
    })
}

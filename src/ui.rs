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
    // Keep 64-bit SQLite IDs lossless in JavaScript. Topcoat escapes the entire
    // JSON attribute, so user text can never terminate an inline script.
    let todos: Vec<_> = todos
        .into_iter()
        .map(|todo| {
            serde_json::json!({
                "id": todo.id.to_string(),
                "title": todo.title,
                "completed": todo.completed,
            })
        })
        .collect();
    let state =
        serde_json::json!({ "todos": todos, "filter": filter.value(), "error": error }).to_string();
    Ok(view! {
        <!DOCTYPE html>
        <html lang="ja" class="dark">
            <head>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                <meta name="theme-color" content="#0a0a0a">
                <meta name="description" content="ひとつずつ、やることを整理するシンプルなTodo。">
                <title>"ひとつずつ。 | Topcoat Todo"</title>
                <link rel="stylesheet" href="/style.css?v=0.2.0">
                <link rel="icon" href="data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 40 40'%3E%3Crect width='40' height='40' rx='10' fill='%230a0a0a'/%3E%3Cpath d='m11 20 6 6 12-13' fill='none' stroke='white' stroke-width='3'/%3E%3C/svg%3E">
            </head>
            <body>
                <div id="root" data-initial-state=(state)></div>
                <noscript><p>"この画面を使うにはJavaScriptを有効にしてください。保存済みのTodoはそのまま保持されています。"</p></noscript>
                <script type="module" src="/app.js?v=0.2.0"></script>
            </body>
        </html>
    })
}

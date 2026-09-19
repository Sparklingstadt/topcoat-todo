use crate::{
    db::Todo,
    model::title_matches,
    routes::{EditDraft, Filter, ListState},
};
use topcoat::{
    Result,
    view::{View, component, view},
};

#[component]
async fn page_head() -> Result<impl View> {
    Ok(view! {
        <head>
            <meta charset="utf-8">
            <meta name="viewport" content="width=device-width, initial-scale=1">
            <meta name="theme-color" content="#f6f5f1">
            <meta name="description" content="ひとつずつ、やることを整理するシンプルなTodo。">
            <title>"ひとつずつ。 | Topcoat Todo"</title>
            <link rel="stylesheet" href="/style.css">
            <link rel="icon" href="data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 40 40'%3E%3Crect width='40' height='40' rx='12' fill='%23344339'/%3E%3Cpath d='m11 20 6 6 12-13' fill='none' stroke='%23fff' stroke-width='3'/%3E%3C/svg%3E">
        </head>
    })
}

#[component]
async fn brand() -> Result<impl View> {
    Ok(view! {
        <header class="brand-bar">
            <a class="brand" href="/" aria-label="ひとつずつ。 ホーム"><span class="brand-icon" aria-hidden="true">icon(kind: "check")</span><span>"ひとつずつ。"</span></a>
            <span class="private-note"><span class="status-dot"></span>"マイリスト"</span>
        </header>
    })
}

#[component]
async fn footer() -> Result<impl View> {
    Ok(
        view! { <footer class="app-footer"><span>"小さな一歩を、積み重ねよう。"</span><span>"TOPCOAT TODO"</span></footer> },
    )
}

#[component]
async fn hidden_state(state: ListState) -> Result<impl View> {
    Ok(
        view! { <input type="hidden" name="filter" value=(state.filter.value())><input type="hidden" name="q" value=(&state.query)> },
    )
}

#[component]
async fn priority_field(value: String, id: &'static str) -> Result<impl View> {
    Ok(view! {
        <div class="form-field"><label for=(id)>"優先度"</label>
            <select id=(id) name="priority">
                <option value="high" selected=(value == "high")>"高"</option>
                <option value="normal" selected=(value == "normal" || value.is_empty())>"通常"</option>
                <option value="low" selected=(value == "low")>"低"</option>
            </select>
        </div>
    })
}

#[component]
async fn date_field(value: String, id: &'static str) -> Result<impl View> {
    Ok(view! {
        <label class="form-field" for=(id)><span>"期限" <small>"任意"</small></span>
            <input id=(id) name="due_date" type="date" min="0001-01-01" max="9999-12-31" value=(value)>
        </label>
    })
}

#[component]
async fn add_form(state: ListState) -> Result<impl View> {
    Ok(view! {
        <form class="add-form" action="/todos/add" method="post">
            hidden_state(state: state)
            <div class="add-main">
                <label class="sr-only" for="todo-title">"新しいTodo"</label>
                <span class="input-plus" aria-hidden="true">"＋"</span>
                <input id="todo-title" name="title" type="text" required=(true) maxlength="200" autocomplete="off" placeholder="次にやることを入力…">
                <button class="add-button" type="submit">"追加" icon(kind: "arrow")</button>
            </div>
            <details class="add-options"><summary>"期限・優先度を設定"</summary>
                <div class="field-grid">date_field(value: String::new(), id: "add-due") priority_field(value: "normal".into(), id: "add-priority")</div>
            </details>
        </form>
    })
}

#[component]
async fn search_form(state: ListState) -> Result<impl View> {
    let clear = ListState {
        filter: state.filter,
        query: String::new(),
    }
    .url();
    Ok(view! {
        <form class="search-form" action="/" method="get" role="search">
            <input type="hidden" name="filter" value=(state.filter.value())>
            <label class="sr-only" for="todo-search">"Todoを検索"</label>
            <input id="todo-search" name="q" type="search" value=(&state.query) maxlength="200" placeholder="Todoを検索…">
            <button class="secondary-button" type="submit">"検索"</button>
            if !state.query.is_empty() { <a class="text-link" href=(clear)>"検索を解除"</a> }
        </form>
    })
}

#[component]
pub async fn document(
    todos: Vec<Todo>,
    state: ListState,
    error: Option<&'static str>,
) -> Result<impl View> {
    let total = todos.len();
    let completed = todos.iter().filter(|todo| todo.completed).count();
    let active = total - completed;
    let percent = (completed * 100).checked_div(total).unwrap_or(0);
    let matches: Vec<_> = todos
        .into_iter()
        .filter(|todo| title_matches(&todo.title, &state.query))
        .collect();
    let matched_total = matches.len();
    let matched_completed = matches.iter().filter(|todo| todo.completed).count();
    let visible: Vec<_> = matches
        .into_iter()
        .filter(|todo| match state.filter {
            Filter::All => true,
            Filter::Active => !todo.completed,
            Filter::Completed => todo.completed,
        })
        .collect();
    Ok(view! {
        <!DOCTYPE html><html lang="ja">page_head()
            <body>
                <a class="skip-link" href="#main">"Todoへ移動"</a>
                <div class="app-shell">brand()
                    <main id="main">
                        <section class="overview" aria-labelledby="page-title">
                            <div><p class="eyebrow">"MY EVERYDAY"</p><h1 id="page-title">"やることリスト"</h1>
                                <p class="summary">if total == 0 { "頭の中のやることを、ここに。" } else if active == 0 { "すべて完了。おつかれさまでした！" } else { "あと " <strong>(active)</strong> " 件。ひとつずつ進めましょう。" }</p>
                            </div>
                            <div class="progress-card" role="group" aria-label=(format!("{}件中{}件完了", total, completed))>
                                <span class="progress-label">"完了したTodo"</span><div class="progress-count"><strong>(completed)</strong><span>"/ " (total)</span></div>
                                <div class="progress-track"><span style=(format!("width: {percent}%"))></span></div>
                            </div>
                        </section>
                        <section class="todo-panel" aria-label="Todoの管理">
                            add_form(state: state.clone())
                            if let Some(message) = error { <p class="error-message" role="alert">(message)</p> }
                            search_form(state: state.clone())
                            if !state.query.is_empty() { <p class="search-result">"「" (&state.query) "」の検索結果：" (matched_total) "件"</p> }
                            <div class="list-toolbar">
                                <nav class="filters" aria-label="Todoの絞り込み">
                                    filter_link(label: "すべて", count: matched_total, value: Filter::All, state: state.clone())
                                    filter_link(label: "未完了", count: matched_total - matched_completed, value: Filter::Active, state: state.clone())
                                    filter_link(label: "完了", count: matched_completed, value: Filter::Completed, state: state.clone())
                                </nav><span class="list-count">(visible.len()) " 件"</span>
                            </div>
                            <p class="sort-hint">"未完了を先に、優先度・期限の順で表示"</p>
                            if visible.is_empty() { empty_state(filter: state.filter, total: total, searching: !state.query.is_empty()) }
                            else { <ul class="todo-list">for todo in visible { todo_row(todo: todo, state: state.clone()) }</ul> }
                            if matched_completed > 0 && state.filter != Filter::Active {
                                <div class="bulk-bar"><a class="text-link" href=(format!("/todos/clear-completed?{}", state.query_string()))>"完了済み " (matched_completed) " 件を削除"</a></div>
                            }
                            <div class="panel-footer"><span>icon(kind: "check") "チェックして完了"</span><span>"自動保存"</span></div>
                        </section>
                    </main>footer()
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
    state: ListState,
) -> Result<impl View> {
    let selected = value == state.filter;
    let href = ListState {
        filter: value,
        query: state.query,
    }
    .url();
    Ok(view! { <a href=(href) if selected { aria-current="page" }>(label)<span>(count)</span></a> })
}

#[component]
async fn todo_row(todo: Todo, state: ListState) -> Result<impl View> {
    Ok(view! {
        <li class=(if todo.completed { "todo-row is-complete" } else { "todo-row" }) data-todo-id=(todo.id)>
            <form class="complete-form" action="/todos/complete" method="post">
                <input type="hidden" name="id" value=(todo.id)><input type="hidden" name="completed" value=((!todo.completed).to_string())>
                hidden_state(state: state.clone())
                <button class="complete-button" type="submit" aria-pressed=(todo.completed.to_string()) aria-label=(format!("{}を{}", todo.title, if todo.completed { "未完了に戻す" } else { "完了にする" }))>
                    <span class="checkbox" aria-hidden="true">if todo.completed { icon(kind: "check") }</span>
                    <span class="todo-text"><span class="todo-title">(&todo.title)</span>
                        <span class="todo-meta"><span class=(format!("priority priority-{}", todo.priority.value()))>"優先度：" (todo.priority.label())</span>
                            if let Some(date) = &todo.due_date { <time datetime=(date)>"期限 " (date.replace('-', "/"))</time> }
                        </span>
                    </span>
                </button>
            </form>
            <a class="edit-link" href=(format!("/todos/edit?id={}&{}", todo.id, state.query_string())) aria-label=(format!("{}を編集", todo.title))>"編集"</a>
            <form class="delete-form" action="/todos/delete" method="post">
                <input type="hidden" name="id" value=(todo.id)>hidden_state(state: state)
                <button class="delete-button" type="submit" aria-label=(format!("{}を削除", todo.title)) title="削除">icon(kind: "trash")</button>
            </form>
        </li>
    })
}

#[component]
pub async fn editor(
    draft: EditDraft,
    state: ListState,
    error: Option<&'static str>,
) -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html><html lang="ja">page_head()<body><div class="app-shell">brand()
            <main id="main" class="subpage">
                <a class="back-link" href=(state.url())>"← 一覧へ戻る"</a>
                <p class="eyebrow">"EDIT YOUR TODO"</p><h1>"Todoを編集"</h1><p class="summary">"やることや期限を、今の予定に合わせて。"</p>
                <section class="editor-panel" aria-label="Todoの編集">
                    if let Some(message) = error { <p class="error-message" role="alert">(message)</p> }
                    <form class="editor-form" action="/todos/update" method="post">
                        <input type="hidden" name="id" value=(draft.id)>hidden_state(state: state.clone())
                        <label class="form-field" for="edit-title"><span>"Todoの内容"</span><input id="edit-title" name="title" type="text" value=(&draft.title) required=(true) maxlength="200" autocomplete="off"></label>
                        <div class="field-grid">date_field(value: draft.due_date, id: "edit-due") priority_field(value: draft.priority, id: "edit-priority")</div>
                        <p class="form-note">"期限を空欄にすると、期限なしに戻せます。完了状態は変わりません。"</p>
                        <div class="form-actions"><a class="secondary-button" href=(state.url())>"キャンセル"</a><button class="add-button" type="submit">"変更を保存"</button></div>
                    </form>
                </section>
            </main>footer()</div></body></html>
    })
}

#[component]
pub async fn confirm_clear(todos: Vec<Todo>, state: ListState) -> Result<impl View> {
    let count = todos.len();
    let ids = todos
        .iter()
        .map(|todo| todo.id.to_string())
        .collect::<Vec<_>>()
        .join(",");
    Ok(view! {
        <!DOCTYPE html><html lang="ja">page_head()<body><div class="app-shell">brand()
            <main id="main" class="subpage">
                <a class="back-link" href=(state.url())>"← 一覧へ戻る"</a><p class="eyebrow">"A FRESH START"</p><h1>"完了済みのTodoを削除"</h1>
                <section class="editor-panel confirm-panel" aria-label="一括削除の確認">
                    if count == 0 { <h2>"削除する完了済みTodoはありません"</h2><a class="secondary-button" href=(state.url())>"一覧へ戻る"</a> }
                    else {
                        <h2>(count) " 件を削除しますか？"</h2>
                        <p>"下に表示したTodoだけが対象です。削除すると元に戻せません。"</p>
                        if !state.query.is_empty() { <p>"検索条件：「" (&state.query) "」"</p> }
                        <ul class="confirmation-list">for todo in todos { <li>icon(kind: "check") <span>(&todo.title)</span></li> }</ul>
                        <form action="/todos/clear-completed" method="post">
                            hidden_state(state: state.clone())<input type="hidden" name="ids" value=(ids)>
                            <label class="confirm-check"><input type="checkbox" name="confirmation" value="delete" required=(true)><span>"上記の完了Todoを削除することを確認しました"</span></label>
                            <p class="form-note">"確認後に未完了へ戻したTodoや、新たに完了したTodoは削除しません。一度に最大1,000件まで削除できます。"</p>
                            <div class="form-actions"><a class="secondary-button" href=(state.url())>"キャンセル"</a><button class="danger-button" type="submit">(count) " 件を削除する"</button></div>
                        </form>
                    }
                </section>
            </main>footer()</div></body></html>
    })
}

#[component]
async fn empty_state(filter: Filter, total: usize, searching: bool) -> Result<impl View> {
    let (title, hint) = if searching {
        (
            "条件に合うTodoがありません",
            "検索する言葉や、表示する状態を変えてみてください。",
        )
    } else {
        match filter {
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
        }
    };
    Ok(view! {
        <div class="empty-state"><div class="empty-illustration" aria-hidden="true">
            <div class="paper"><span class="paper-line"></span><span class="paper-line"></span><span class="paper-line short"></span></div>
            <span class="paper-check">icon(kind: "check")</span><span class="spark spark-one">"＋"</span><span class="spark spark-two">"·"</span>
        </div><h2>(title)</h2><p>(hint)</p></div>
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

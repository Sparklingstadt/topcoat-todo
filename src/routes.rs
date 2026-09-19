use crate::{
    db::Database,
    model::{Priority, title_matches, valid_date, validated_title},
    ui,
};
use serde::Deserialize;
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{
        StatusCode,
        content::{Css, Form},
        page, query_params,
        response::{AsyncIntoResponse, IntoResponse, Response},
        route,
    },
    view::{View, ViewExt, view},
};

#[derive(Clone, Copy, Default, PartialEq)]
pub enum Filter {
    #[default]
    All,
    Active,
    Completed,
}
impl Filter {
    pub fn from_str(value: &str) -> Self {
        match value {
            "active" => Self::Active,
            "completed" => Self::Completed,
            _ => Self::All,
        }
    }
    pub fn value(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Active => "active",
            Self::Completed => "completed",
        }
    }
}

#[derive(Clone, Default)]
pub struct ListState {
    pub filter: Filter,
    pub query: String,
}
impl ListState {
    pub fn new(filter: &str, query: &str) -> Self {
        Self {
            filter: Filter::from_str(filter),
            query: query.trim().chars().take(200).collect(),
        }
    }
    pub fn query_string(&self) -> String {
        let mut result = format!("filter={}", self.filter.value());
        if !self.query.is_empty() {
            result.push_str("&q=");
            result.push_str(&encode_query(&self.query));
        }
        result
    }
    pub fn url(&self) -> String {
        format!("/?{}", self.query_string())
    }
}

fn encode_query(value: &str) -> String {
    value
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
                (byte as char).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}

#[query_params(error = bad_request)]
struct PageQuery {
    filter: Option<String>,
    q: Option<String>,
    error: Option<String>,
    id: Option<i64>,
}
impl PageQuery {
    fn state(&self) -> ListState {
        ListState::new(
            self.filter.as_deref().unwrap_or("all"),
            self.q.as_deref().unwrap_or(""),
        )
    }
}

fn error_message(code: &str) -> &'static str {
    match code {
        "title" => "Todoは空白以外の文字を含む、200文字以内で入力してください。",
        "date" => "期限には0001年から9999年までの実在する日付を入力してください。",
        "priority" => "優先度は「高」「通常」「低」から選んでください。",
        "missing" => "このTodoはすでに削除されています。",
        _ => "保存できませんでした。しばらく待って、もう一度お試しください。",
    }
}

#[page("/")]
async fn home(cx: &Cx) -> Result<impl View> {
    let query = query_params::<PageQuery>(cx)?;
    let todos = app_context::<Database>(cx).list().await?;
    let error = query.error.as_deref().map(error_message);
    Ok(view! { ui::document(todos: todos, state: query.state(), error: error) })
}

#[derive(Deserialize)]
struct SaveTodo {
    id: Option<i64>,
    title: String,
    #[serde(default)]
    due_date: String,
    #[serde(default)]
    priority: String,
    #[serde(default)]
    filter: String,
    #[serde(default)]
    q: String,
}

#[derive(Clone)]
pub struct EditDraft {
    pub id: i64,
    pub title: String,
    pub due_date: String,
    pub priority: String,
}

fn validated_fields(
    title: &str,
    date: &str,
    priority: &str,
) -> std::result::Result<(String, Option<String>, Priority), &'static str> {
    let title = validated_title(title).ok_or("title")?;
    if !valid_date(date) {
        return Err("date");
    }
    let priority = Priority::parse(priority).ok_or("priority")?;
    Ok((
        title.to_owned(),
        (!date.is_empty()).then(|| date.to_owned()),
        priority,
    ))
}

#[derive(Deserialize)]
struct ChangeTodo {
    id: i64,
    completed: bool,
    #[serde(default)]
    filter: String,
    #[serde(default)]
    q: String,
}
#[derive(Deserialize)]
struct DeleteTodo {
    id: i64,
    #[serde(default)]
    filter: String,
    #[serde(default)]
    q: String,
}
#[derive(Deserialize)]
struct ClearCompleted {
    ids: String,
    #[serde(default)]
    confirmation: String,
    #[serde(default)]
    filter: String,
    #[serde(default)]
    q: String,
}

type Redirect = (StatusCode, [(&'static str, String); 1], &'static str);
fn redirect(state: &ListState, error: Option<&str>) -> Redirect {
    let mut location = state.url();
    if let Some(error) = error {
        location.push_str("&error=");
        location.push_str(error);
    }
    (StatusCode::SEE_OTHER, [("location", location)], "")
}

#[route(POST "/todos/add")]
async fn add(cx: &Cx, Form(input): Form<SaveTodo>) -> Result<Redirect> {
    let mut state = ListState::new(&input.filter, &input.q);
    let (title, date, priority) =
        match validated_fields(&input.title, &input.due_date, &input.priority) {
            Ok(fields) => fields,
            Err(code) => return Ok(redirect(&state, Some(code))),
        };
    if let Err(error) = app_context::<Database>(cx)
        .add_details(title, date, priority)
        .await
    {
        eprintln!("Could not add todo: {error}");
        return Ok(redirect(&state, Some("save")));
    }
    if state.filter == Filter::Completed {
        state.filter = Filter::All;
    }
    // Reveal the newly created Todo even when it does not match the old search.
    state.query.clear();
    Ok(redirect(&state, None))
}

#[route(GET "/todos/edit")]
async fn edit(cx: &Cx) -> Result<Response> {
    let query = query_params::<PageQuery>(cx)?;
    let state = query.state();
    let todo = app_context::<Database>(cx)
        .list()
        .await?
        .into_iter()
        .find(|todo| Some(todo.id) == query.id);
    let Some(todo) = todo else {
        return redirect(&state, Some("missing")).into_response(cx);
    };
    let draft = EditDraft {
        id: todo.id,
        title: todo.title,
        due_date: todo.due_date.unwrap_or_default(),
        priority: todo.priority.value().to_owned(),
    };
    edit_response(cx, draft, state, None, StatusCode::OK).await
}

async fn edit_response(
    cx: &Cx,
    draft: EditDraft,
    state: ListState,
    error: Option<&'static str>,
    status: StatusCode,
) -> Result<Response> {
    let mut response = view! { cx => ui::editor(draft: draft, state: state, error: error) }
        .boxed()
        .async_into_response(cx)
        .await?;
    *response.status_mut() = status;
    Ok(response)
}

#[route(POST "/todos/update")]
async fn update(cx: &Cx, Form(input): Form<SaveTodo>) -> Result<Response> {
    let state = ListState::new(&input.filter, &input.q);
    let Some(id) = input.id else {
        return (StatusCode::BAD_REQUEST, "TodoのIDが必要です。").into_response(cx);
    };
    let (title, date, priority) =
        match validated_fields(&input.title, &input.due_date, &input.priority) {
            Ok(fields) => fields,
            Err(code) => {
                return edit_response(
                    cx,
                    EditDraft {
                        id,
                        title: input.title,
                        due_date: input.due_date,
                        priority: input.priority,
                    },
                    state,
                    Some(error_message(code)),
                    StatusCode::BAD_REQUEST,
                )
                .await;
            }
        };
    let result = app_context::<Database>(cx)
        .update(id, title, date, priority)
        .await;
    // Validation failures preserve the submitted draft; successful updates keep
    // the original search/filter so returning to a list remains predictable.
    mutation_redirect(&state, result).into_response(cx)
}

#[route(POST "/todos/complete")]
async fn complete(cx: &Cx, Form(input): Form<ChangeTodo>) -> Result<Redirect> {
    let result = app_context::<Database>(cx)
        .set_completed(input.id, input.completed)
        .await;
    Ok(mutation_redirect(
        &ListState::new(&input.filter, &input.q),
        result,
    ))
}

#[route(POST "/todos/delete")]
async fn delete(cx: &Cx, Form(input): Form<DeleteTodo>) -> Result<Redirect> {
    let result = app_context::<Database>(cx).delete(input.id).await;
    Ok(mutation_redirect(
        &ListState::new(&input.filter, &input.q),
        result,
    ))
}

#[page("/todos/clear-completed")]
async fn confirm_clear(cx: &Cx) -> Result<impl View> {
    let query = query_params::<PageQuery>(cx)?;
    let state = query.state();
    let todos: Vec<_> = app_context::<Database>(cx)
        .list()
        .await?
        .into_iter()
        .filter(|todo| todo.completed && title_matches(&todo.title, &state.query))
        .take(1000)
        .collect();
    Ok(view! { ui::confirm_clear(todos: todos, state: state) })
}

fn confirmed_ids(value: &str) -> Option<Vec<i64>> {
    if value.is_empty() || value.len() > 20_000 {
        return None;
    }
    let ids: Option<Vec<_>> = value
        .split(',')
        .map(|id| id.parse::<i64>().ok().filter(|id| *id > 0))
        .collect();
    ids.filter(|ids| ids.len() <= 1000)
}

#[route(POST "/todos/clear-completed")]
async fn clear(cx: &Cx, Form(input): Form<ClearCompleted>) -> Result<Response> {
    let Some(ids) = confirmed_ids(&input.ids).filter(|_| input.confirmation == "delete") else {
        return (
            StatusCode::BAD_REQUEST,
            "削除対象を確認し、確認欄にチェックを入れてください。",
        )
            .into_response(cx);
    };
    let result = app_context::<Database>(cx)
        .delete_completed(ids)
        .await
        .map(|_| true);
    mutation_redirect(&ListState::new(&input.filter, &input.q), result).into_response(cx)
}

fn mutation_redirect(state: &ListState, result: Result<bool>) -> Redirect {
    match result {
        Ok(true) => redirect(state, None),
        Ok(false) => redirect(state, Some("missing")),
        Err(error) => {
            eprintln!("Could not update todo: {error}");
            redirect(state, Some("save"))
        }
    }
}

#[route(GET "/style.css")]
async fn stylesheet() -> Result<Css<&'static str>> {
    Ok(Css(include_str!("../assets/style.css")))
}
#[route(GET "/health")]
async fn health(cx: &Cx) -> Result<&'static str> {
    app_context::<Database>(cx).list().await?;
    Ok("ok")
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_rejects_empty_and_unicode_whitespace() {
        for input in ["", "  ", "\t\r\n", "　", "\u{2003}\u{00a0}"] {
            assert_eq!(validated_title(input), None, "input: {input:?}");
        }
    }

    #[test]
    fn title_trims_edges_but_preserves_internal_spaces() {
        assert_eq!(
            validated_title("　 買い物 と 散歩 \n"),
            Some("買い物 と 散歩")
        );
    }

    #[test]
    fn title_limit_counts_unicode_characters_not_bytes() {
        for character in ['a', 'あ', '🍅'] {
            let maximum = character.to_string().repeat(200);
            assert_eq!(validated_title(&maximum), Some(maximum.as_str()));
            assert_eq!(validated_title(&format!("{maximum}{character}")), None);
        }
    }

    #[test]
    fn title_limit_is_applied_after_trimming() {
        let maximum = "あ".repeat(200);
        let padded = format!("　{maximum} \n");
        assert_eq!(validated_title(&padded), Some(maximum.as_str()));
    }

    #[test]
    fn unknown_filters_fall_back_to_all() {
        for input in [
            "",
            "unknown",
            "ACTIVE",
            "https://example.com",
            "all&error=save",
        ] {
            assert!(Filter::from_str(input) == Filter::All);
        }
        assert!(Filter::from_str("active") == Filter::Active);
        assert!(Filter::from_str("completed") == Filter::Completed);
    }

    #[test]
    fn mutation_redirects_use_post_redirect_get_and_keep_the_filter() {
        let success = mutation_redirect(&ListState::new("active", ""), Ok(true));
        assert_eq!(success.0, StatusCode::SEE_OTHER);
        assert_eq!(success.1, [("location", "/?filter=active".into())]);
        assert_eq!(success.2, "");

        let missing = mutation_redirect(&ListState::new("completed", ""), Ok(false));
        assert_eq!(missing.1[0].1, "/?filter=completed&error=missing");
    }

    #[test]
    fn storage_errors_do_not_expose_internal_details_in_redirects() {
        let error = std::io::Error::other("private database path");
        let response = mutation_redirect(&ListState::new("all", ""), Err(error.into()));
        assert_eq!(response.0, StatusCode::SEE_OTHER);
        assert_eq!(response.1[0].1, "/?filter=all&error=save");
    }
    #[test]
    fn query_parameters_cannot_escape_the_search_value() {
        let state = ListState::new("completed", "Rust &filter=all+日本語/#?");
        assert_eq!(state.query_string().matches('&').count(), 1);
        assert!(
            state
                .url()
                .starts_with("/?filter=completed&q=Rust%20%26filter%3Dall%2B")
        );
        assert!(state.url().ends_with("%2F%23%3F"));
        assert_eq!(ListState::new("unknown", "　 Rust　 ").query, "Rust");
        assert_eq!(
            ListState::new("all", &"あ".repeat(201))
                .query
                .chars()
                .count(),
            200
        );
    }

    #[test]
    fn form_validation_rejects_bad_dates_and_priorities() {
        assert_eq!(
            validated_fields("タイトル", "2026-02-30", "normal").unwrap_err(),
            "date"
        );
        assert_eq!(
            validated_fields("タイトル", "", "urgent").unwrap_err(),
            "priority"
        );
        let fields = validated_fields("　タイトル　", "2028-02-29", "high").unwrap();
        assert_eq!(
            fields,
            ("タイトル".into(), Some("2028-02-29".into()), Priority::High)
        );
    }

    #[test]
    fn bulk_selection_requires_positive_bounded_ids() {
        assert_eq!(
            confirmed_ids("1,2,9223372036854775807"),
            Some(vec![1, 2, i64::MAX])
        );
        for value in ["", "-1", "0", "1,", "x", "1 OR 1=1", "9223372036854775808"] {
            assert_eq!(confirmed_ids(value), None);
        }
        assert!(confirmed_ids(&vec!["1"; 1001].join(",")).is_none());
    }
}

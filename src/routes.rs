use serde::Deserialize;
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{
        StatusCode,
        content::{Css, Form},
        page, query_params, route,
    },
    view::{View, view},
};

use crate::{db::Database, ui};

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

#[query_params(error = bad_request)]
struct PageQuery {
    filter: Option<String>,
    error: Option<String>,
}

#[page("/")]
async fn home(cx: &Cx) -> Result<impl View> {
    let query = query_params::<PageQuery>(cx)?;
    let filter = Filter::from_str(query.filter.as_deref().unwrap_or("all"));
    let todos = app_context::<Database>(cx).list().await?;
    let error = match query.error.as_deref() {
        Some("title") => Some("Todoは空白以外の文字を含む、200文字以内で入力してください。"),
        Some("missing") => Some("このTodoはすでに削除されています。"),
        Some("save") => Some("保存できませんでした。しばらく待って、もう一度お試しください。"),
        _ => None,
    };
    Ok(view! { ui::document(todos: todos, filter: filter, error: error) })
}

#[derive(Deserialize)]
struct AddTodo {
    title: String,
    #[serde(default)]
    filter: String,
}

#[derive(Deserialize)]
struct ChangeTodo {
    id: i64,
    completed: bool,
    #[serde(default)]
    filter: String,
}

#[derive(Deserialize)]
struct DeleteTodo {
    id: i64,
    #[serde(default)]
    filter: String,
}

type Redirect = (StatusCode, [(&'static str, String); 1], &'static str);

fn redirect(filter: Filter, error: Option<&str>) -> Redirect {
    let mut location = format!("/?filter={}", filter.value());
    if let Some(error) = error {
        location.push_str("&error=");
        location.push_str(error);
    }
    (StatusCode::SEE_OTHER, [("location", location)], "")
}

fn validated_title(input: &str) -> Option<&str> {
    let title = input.trim();
    (!title.is_empty() && title.chars().count() <= 200).then_some(title)
}

#[route(POST "/todos/add")]
async fn add(cx: &Cx, Form(input): Form<AddTodo>) -> Result<Redirect> {
    let filter = Filter::from_str(&input.filter);
    let Some(title) = validated_title(&input.title) else {
        return Ok(redirect(filter, Some("title")));
    };
    if let Err(error) = app_context::<Database>(cx).add(title.to_owned()).await {
        eprintln!("Could not add todo: {error}");
        return Ok(redirect(filter, Some("save")));
    }
    // A new item is incomplete, so make it visible even from the completed tab.
    let destination = if filter == Filter::Completed {
        Filter::All
    } else {
        filter
    };
    Ok(redirect(destination, None))
}

#[route(POST "/todos/complete")]
async fn complete(cx: &Cx, Form(input): Form<ChangeTodo>) -> Result<Redirect> {
    let result = app_context::<Database>(cx)
        .set_completed(input.id, input.completed)
        .await;
    Ok(mutation_redirect(Filter::from_str(&input.filter), result))
}

#[route(POST "/todos/delete")]
async fn delete(cx: &Cx, Form(input): Form<DeleteTodo>) -> Result<Redirect> {
    let result = app_context::<Database>(cx).delete(input.id).await;
    Ok(mutation_redirect(Filter::from_str(&input.filter), result))
}

fn mutation_redirect(filter: Filter, result: Result<bool>) -> Redirect {
    match result {
        Ok(true) => redirect(filter, None),
        Ok(false) => redirect(filter, Some("missing")),
        Err(error) => {
            eprintln!("Could not update todo: {error}");
            redirect(filter, Some("save"))
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
        let success = mutation_redirect(Filter::Active, Ok(true));
        assert_eq!(success.0, StatusCode::SEE_OTHER);
        assert_eq!(success.1, [("location", "/?filter=active".into())]);
        assert_eq!(success.2, "");

        let missing = mutation_redirect(Filter::Completed, Ok(false));
        assert_eq!(missing.1[0].1, "/?filter=completed&error=missing");
    }

    #[test]
    fn storage_errors_do_not_expose_internal_details_in_redirects() {
        let error = std::io::Error::other("private database path");
        let response = mutation_redirect(Filter::All, Err(error.into()));
        assert_eq!(response.0, StatusCode::SEE_OTHER);
        assert_eq!(response.1[0].1, "/?filter=all&error=save");
    }
}

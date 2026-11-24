use crate::models::{Category, ItemSummary};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// Порядок выдачи записей. Вычисляется в Rust, чтобы frontend получал уже
/// готовый список и не тратил циклы на пересортировку больших хранилищ.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum SortMode {
    #[default]
    Recent,
    Title,
    Category,
}

#[derive(Debug, Clone, Default)]
pub struct ItemFilter {
    pub query: String,
    pub category: Option<Category>,
    pub favorites_only: bool,
}

impl ItemFilter {
    pub fn new(
        query: Option<String>,
        category: Option<Category>,
        favorites_only: Option<bool>,
    ) -> Self {
        Self {
            query: query.unwrap_or_default().trim().to_lowercase(),
            category,
            favorites_only: favorites_only.unwrap_or(false),
        }
    }

    fn matches(&self, item: &ItemSummary) -> bool {
        if self.favorites_only && !item.favorite {
            return false;
        }
        if let Some(expected) = self.category {
            if item.category != expected {
                return false;
            }
        }
        if self.query.is_empty() {
            return true;
        }

        let mut haystack = String::with_capacity(128);
        haystack.push_str(&item.title);
        haystack.push(' ');
        haystack.push_str(&item.service);
        haystack.push(' ');
        haystack.push_str(&item.username);
        haystack.push(' ');
        haystack.push_str(item.category.as_str());
        for tag in &item.tags {
            haystack.push(' ');
            haystack.push_str(tag);
        }
        haystack.to_lowercase().contains(&self.query)
    }
}

fn compare_recent(left: &ItemSummary, right: &ItemSummary) -> Ordering {
    right
        .favorite
        .cmp(&left.favorite)
        .then_with(|| right.updated_at.cmp(&left.updated_at))
        .then_with(|| left.id.cmp(&right.id))
}

fn compare_title(left: &ItemSummary, right: &ItemSummary) -> Ordering {
    left.title
        .to_lowercase()
        .cmp(&right.title.to_lowercase())
        .then_with(|| left.id.cmp(&right.id))
}

fn compare_category(left: &ItemSummary, right: &ItemSummary) -> Ordering {
    left.category
        .as_str()
        .cmp(right.category.as_str())
        .then_with(|| left.title.to_lowercase().cmp(&right.title.to_lowercase()))
        .then_with(|| left.id.cmp(&right.id))
}

/// Фильтрует и сортирует уже расшифрованные сводки записей.
pub fn apply_filter(
    items: Vec<ItemSummary>,
    filter: &ItemFilter,
    sort: SortMode,
) -> Vec<ItemSummary> {
    let mut result: Vec<ItemSummary> = items
        .into_iter()
        .filter(|item| filter.matches(item))
        .collect();

    match sort {
        SortMode::Recent => result.sort_by(compare_recent),
        SortMode::Title => result.sort_by(compare_title),
        SortMode::Category => result.sort_by(compare_category),
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn summary(
        id: &str,
        category: Category,
        title: &str,
        service: &str,
        username: &str,
        favorite: bool,
        tags: &[&str],
        updated_at: &str,
    ) -> ItemSummary {
        ItemSummary {
            id: id.to_string(),
            category,
            title: title.to_string(),
            service: service.to_string(),
            username: username.to_string(),
            favorite,
            tags: tags.iter().map(|tag| tag.to_string()).collect(),
            has_password: true,
            has_totp: false,
            totp: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: updated_at.to_string(),
        }
    }

    fn fixtures() -> Vec<ItemSummary> {
        vec![
            summary(
                "a",
                Category::Login,
                "Почта",
                "https://mail.example",
                "user@example.com",
                false,
                &["работа"],
                "2026-01-03T10:00:00Z",
            ),
            summary(
                "b",
                Category::ApiKey,
                "CI токен",
                "https://ci.example",
                "deploy-bot",
                true,
                &["devops", "primary"],
                "2026-01-02T10:00:00Z",
            ),
            summary(
                "c",
                Category::Ssh,
                "Сервер",
                "ssh://edge",
                "root",
                false,
                &["infra"],
                "2026-01-04T10:00:00Z",
            ),
        ]
    }

    #[test]
    fn empty_filter_keeps_every_item() {
        let result = apply_filter(fixtures(), &ItemFilter::default(), SortMode::Recent);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn query_matches_title_service_username_and_tags() {
        for needle in ["почта", "mail.example", "user@example", "работа", "LOGIN"] {
            let filter = ItemFilter::new(Some(needle.to_string()), None, None);
            let result = apply_filter(fixtures(), &filter, SortMode::Recent);
            assert_eq!(result.len(), 1, "needle: {needle}");
            assert_eq!(result[0].id, "a");
        }
    }

    #[test]
    fn category_and_favorite_filters_compose() {
        let filter = ItemFilter::new(None, Some(Category::ApiKey), Some(true));
        let result = apply_filter(fixtures(), &filter, SortMode::Recent);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "b");

        let impossible = ItemFilter::new(None, Some(Category::Card), None);
        assert!(apply_filter(fixtures(), &impossible, SortMode::Recent).is_empty());
    }

    #[test]
    fn recent_sort_puts_favorites_first_then_newest() {
        let result = apply_filter(fixtures(), &ItemFilter::default(), SortMode::Recent);
        let ids: Vec<&str> = result.iter().map(|item| item.id.as_str()).collect();
        assert_eq!(ids, vec!["b", "c", "a"]);
    }

    #[test]
    fn title_and_category_sorts_are_deterministic() {
        let by_title = apply_filter(fixtures(), &ItemFilter::default(), SortMode::Title);
        assert_eq!(by_title[0].title, "CI токен");

        let by_category = apply_filter(fixtures(), &ItemFilter::default(), SortMode::Category);
        let categories: Vec<&str> = by_category
            .iter()
            .map(|item| item.category.as_str())
            .collect();
        assert_eq!(categories, vec!["apiKey", "login", "ssh"]);
    }
}

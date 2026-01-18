use std::collections::HashMap;

use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct Tag {
    pub id: Uuid,
    pub tag_name: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateTag {
    pub tag_name: String,
    pub content: String,
}

#[derive(Debug, Deserialize, TS)]
pub struct UpdateTag {
    pub tag_name: Option<String>,
    pub content: Option<String>,
}

impl Tag {
    pub async fn find_all(pool: &PgPool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            Tag,
            r#"SELECT id as "id!: Uuid", tag_name, content as "content!", created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>"
               FROM tags
               ORDER BY tag_name ASC"#
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Tag,
            r#"SELECT id as "id!: Uuid", tag_name, content as "content!", created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>"
               FROM tags
               WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn create(pool: &PgPool, data: &CreateTag) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as!(
            Tag,
            r#"INSERT INTO tags (id, tag_name, content)
               VALUES ($1, $2, $3)
               RETURNING id as "id!: Uuid", tag_name, content as "content!", created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            data.tag_name,
            data.content
        )
        .fetch_one(pool)
        .await
    }

    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        data: &UpdateTag,
    ) -> Result<Self, sqlx::Error> {
        let existing = Self::find_by_id(pool, id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;

        let tag_name = data.tag_name.as_ref().unwrap_or(&existing.tag_name);
        let content = data.content.as_ref().unwrap_or(&existing.content);

        sqlx::query_as!(
            Tag,
            r#"UPDATE tags
               SET tag_name = $2, content = $3, updated_at = NOW()
               WHERE id = $1
               RETURNING id as "id!: Uuid", tag_name, content as "content!", created_at as "created_at!: DateTime<Utc>", updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            tag_name,
            content
        )
        .fetch_one(pool)
        .await
    }

    pub async fn delete(pool: &PgPool, id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM tags WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }

    /// Expands @tagname references in text by replacing them with tag content.
    /// Returns the original text if no tags are found or if there's an error.
    /// Unknown tags are left as-is (not expanded, not an error).
    pub async fn expand_tags(pool: &PgPool, text: &str) -> String {
        // Pattern matches @tagname where tagname is non-whitespace, non-@ characters
        let tag_pattern = match Regex::new(r"@([^\s@]+)") {
            Ok(re) => re,
            Err(_) => return text.to_string(),
        };

        // Find all unique tag names referenced in the text
        let tag_names: Vec<String> = tag_pattern
            .captures_iter(text)
            .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        if tag_names.is_empty() {
            return text.to_string();
        }

        // Fetch all tags from the database
        let tags = match Self::find_all(pool).await {
            Ok(t) => t,
            Err(_) => return text.to_string(),
        };

        // Build a map of tag_name -> content for quick lookup
        let tag_map: HashMap<&str, &str> = tags
            .iter()
            .map(|t| (t.tag_name.as_str(), t.content.as_str()))
            .collect();

        // Replace each @tagname with its content (if found)
        let result = tag_pattern.replace_all(text, |caps: &regex::Captures| {
            let tag_name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            match tag_map.get(tag_name) {
                Some(content) => (*content).to_string(),
                None => caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string(),
            }
        });

        result.into_owned()
    }

    /// Helper to expand tags in an Option<String>, returning None if input is None
    pub async fn expand_tags_optional(pool: &PgPool, text: Option<&str>) -> Option<String> {
        match text {
            Some(t) => Some(Self::expand_tags(pool, t).await),
            None => None,
        }
    }
}

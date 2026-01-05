use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Executor, FromRow, Sqlite, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

/// Trigger type for automation rules
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TriggerType {
    OnEnter,
    OnExit,
}

impl TriggerType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TriggerType::OnEnter => "on_enter",
            TriggerType::OnExit => "on_exit",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "on_enter" => Some(TriggerType::OnEnter),
            "on_exit" => Some(TriggerType::OnExit),
            _ => None,
        }
    }
}

/// Action type for automation rules
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    RunAgent,
    CreatePr,
    MergePr,
    Webhook,
    Notify,
}

impl ActionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ActionType::RunAgent => "run_agent",
            ActionType::CreatePr => "create_pr",
            ActionType::MergePr => "merge_pr",
            ActionType::Webhook => "webhook",
            ActionType::Notify => "notify",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "run_agent" => Some(ActionType::RunAgent),
            "create_pr" => Some(ActionType::CreatePr),
            "merge_pr" => Some(ActionType::MergePr),
            "webhook" => Some(ActionType::Webhook),
            "notify" => Some(ActionType::Notify),
            _ => None,
        }
    }
}

/// Configuration for run_agent action
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct RunAgentConfig {
    pub agent_id: Option<Uuid>,
    pub prompt_template: String,
    pub executor: Option<String>,
    pub timeout_minutes: Option<i32>,
}

/// Configuration for create_pr action
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct CreatePrConfig {
    pub title_template: String,
    pub body_template: String,
    pub draft: Option<bool>,
    pub auto_merge_on_approval: Option<bool>,
}

/// Configuration for webhook action
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct WebhookConfig {
    pub url: String,
    pub method: Option<String>,
    pub headers: Option<serde_json::Value>,
    pub body_template: Option<String>,
}

/// Configuration for notify action
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct NotifyConfig {
    pub channel: String,
    pub webhook_url: String,
    pub message_template: String,
}

/// An automation rule that triggers on column entry/exit
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct AutomationRule {
    pub id: Uuid,
    pub project_id: Uuid,
    pub column_id: Uuid,
    pub trigger_type: String,
    pub action_type: String,
    pub action_config: String,
    pub enabled: bool,
    pub priority: i32,
    pub name: Option<String>,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "Date")]
    pub updated_at: DateTime<Utc>,
}

/// Automation rule with column name for UI display
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct AutomationRuleWithColumn {
    pub id: Uuid,
    pub project_id: Uuid,
    pub column_id: Uuid,
    pub column_name: String,
    pub trigger_type: String,
    pub action_type: String,
    pub action_config: String,
    pub enabled: bool,
    pub priority: i32,
    pub name: Option<String>,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "Date")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, TS)]
pub struct CreateAutomationRule {
    pub column_id: Uuid,
    pub trigger_type: TriggerType,
    pub action_type: ActionType,
    pub action_config: serde_json::Value,
    pub enabled: Option<bool>,
    pub priority: Option<i32>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, TS)]
pub struct UpdateAutomationRule {
    pub trigger_type: Option<TriggerType>,
    pub action_type: Option<ActionType>,
    pub action_config: Option<serde_json::Value>,
    pub enabled: Option<bool>,
    pub priority: Option<i32>,
    pub name: Option<String>,
}

impl AutomationRule {
    /// Parse trigger_type string to enum
    pub fn get_trigger_type(&self) -> Option<TriggerType> {
        TriggerType::from_str(&self.trigger_type)
    }

    /// Parse action_type string to enum
    pub fn get_action_type(&self) -> Option<ActionType> {
        ActionType::from_str(&self.action_type)
    }

    /// Parse action_config JSON
    pub fn get_action_config<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_str(&self.action_config)
    }

    /// Find all rules for a project
    pub async fn find_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            AutomationRule,
            r#"SELECT id as "id!: Uuid",
                      project_id as "project_id!: Uuid",
                      column_id as "column_id!: Uuid",
                      trigger_type,
                      action_type,
                      action_config,
                      enabled as "enabled!: bool",
                      priority as "priority!: i32",
                      name,
                      created_at as "created_at!: DateTime<Utc>",
                      updated_at as "updated_at!: DateTime<Utc>"
               FROM automation_rules
               WHERE project_id = $1
               ORDER BY priority ASC"#,
            project_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find all rules with column names for UI display
    pub async fn find_by_project_with_columns(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<AutomationRuleWithColumn>, sqlx::Error> {
        sqlx::query_as!(
            AutomationRuleWithColumn,
            r#"SELECT ar.id as "id!: Uuid",
                      ar.project_id as "project_id!: Uuid",
                      ar.column_id as "column_id!: Uuid",
                      kc.name as "column_name!",
                      ar.trigger_type,
                      ar.action_type,
                      ar.action_config,
                      ar.enabled as "enabled!: bool",
                      ar.priority as "priority!: i32",
                      ar.name,
                      ar.created_at as "created_at!: DateTime<Utc>",
                      ar.updated_at as "updated_at!: DateTime<Utc>"
               FROM automation_rules ar
               JOIN kanban_columns kc ON kc.id = ar.column_id
               WHERE ar.project_id = $1
               ORDER BY ar.priority ASC"#,
            project_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find enabled rules for a column and trigger type (used when task moves)
    pub async fn find_triggered_rules(
        pool: &SqlitePool,
        column_id: Uuid,
        trigger_type: TriggerType,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let trigger_str = trigger_type.as_str();
        sqlx::query_as!(
            AutomationRule,
            r#"SELECT id as "id!: Uuid",
                      project_id as "project_id!: Uuid",
                      column_id as "column_id!: Uuid",
                      trigger_type,
                      action_type,
                      action_config,
                      enabled as "enabled!: bool",
                      priority as "priority!: i32",
                      name,
                      created_at as "created_at!: DateTime<Utc>",
                      updated_at as "updated_at!: DateTime<Utc>"
               FROM automation_rules
               WHERE column_id = $1 AND trigger_type = $2 AND enabled = 1
               ORDER BY priority ASC"#,
            column_id,
            trigger_str
        )
        .fetch_all(pool)
        .await
    }

    /// Find a rule by ID
    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            AutomationRule,
            r#"SELECT id as "id!: Uuid",
                      project_id as "project_id!: Uuid",
                      column_id as "column_id!: Uuid",
                      trigger_type,
                      action_type,
                      action_config,
                      enabled as "enabled!: bool",
                      priority as "priority!: i32",
                      name,
                      created_at as "created_at!: DateTime<Utc>",
                      updated_at as "updated_at!: DateTime<Utc>"
               FROM automation_rules
               WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    /// Create a new automation rule
    pub async fn create<'e, E>(
        executor: E,
        project_id: Uuid,
        data: &CreateAutomationRule,
    ) -> Result<Self, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let id = Uuid::new_v4();
        let trigger_type = data.trigger_type.as_str();
        let action_type = data.action_type.as_str();
        let action_config = data.action_config.to_string();
        let enabled = data.enabled.unwrap_or(true);
        let priority = data.priority.unwrap_or(0);

        sqlx::query_as!(
            AutomationRule,
            r#"INSERT INTO automation_rules (id, project_id, column_id, trigger_type, action_type, action_config, enabled, priority, name)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
               RETURNING id as "id!: Uuid",
                         project_id as "project_id!: Uuid",
                         column_id as "column_id!: Uuid",
                         trigger_type,
                         action_type,
                         action_config,
                         enabled as "enabled!: bool",
                         priority as "priority!: i32",
                         name,
                         created_at as "created_at!: DateTime<Utc>",
                         updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            project_id,
            data.column_id,
            trigger_type,
            action_type,
            action_config,
            enabled,
            priority,
            data.name
        )
        .fetch_one(executor)
        .await
    }

    /// Update an automation rule
    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        data: &UpdateAutomationRule,
    ) -> Result<Self, sqlx::Error> {
        let existing = Self::find_by_id(pool, id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;

        let trigger_type = data
            .trigger_type
            .as_ref()
            .map(|t| t.as_str().to_string())
            .unwrap_or(existing.trigger_type);
        let action_type = data
            .action_type
            .as_ref()
            .map(|t| t.as_str().to_string())
            .unwrap_or(existing.action_type);
        let action_config = data
            .action_config
            .as_ref()
            .map(|c| c.to_string())
            .unwrap_or(existing.action_config);
        let enabled = data.enabled.unwrap_or(existing.enabled);
        let priority = data.priority.unwrap_or(existing.priority);
        let name = data.name.clone().or(existing.name);

        sqlx::query_as!(
            AutomationRule,
            r#"UPDATE automation_rules
               SET trigger_type = $2, action_type = $3, action_config = $4, enabled = $5, priority = $6, name = $7,
                   updated_at = datetime('now', 'subsec')
               WHERE id = $1
               RETURNING id as "id!: Uuid",
                         project_id as "project_id!: Uuid",
                         column_id as "column_id!: Uuid",
                         trigger_type,
                         action_type,
                         action_config,
                         enabled as "enabled!: bool",
                         priority as "priority!: i32",
                         name,
                         created_at as "created_at!: DateTime<Utc>",
                         updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            trigger_type,
            action_type,
            action_config,
            enabled,
            priority,
            name
        )
        .fetch_one(pool)
        .await
    }

    /// Toggle enabled state
    pub async fn set_enabled(
        pool: &SqlitePool,
        id: Uuid,
        enabled: bool,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"UPDATE automation_rules
               SET enabled = $2, updated_at = datetime('now', 'subsec')
               WHERE id = $1"#,
            id,
            enabled
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Delete a rule
    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<u64, sqlx::Error> {
        let result: sqlx::sqlite::SqliteQueryResult =
            sqlx::query!("DELETE FROM automation_rules WHERE id = $1", id)
                .execute(pool)
                .await?;
        Ok(result.rows_affected())
    }
}

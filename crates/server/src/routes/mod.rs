use axum::{
    Router,
    routing::{IntoMakeService, get},
};

use crate::DeploymentImpl;

pub mod agents;
pub mod approvals;
pub mod automation_rules;
pub mod boards;
pub mod config;
pub mod containers;
pub mod context_artifacts;
pub mod debug_events;
pub mod filesystem;
// pub mod github;
pub mod events;
pub mod execution_processes;
pub mod frontend;
pub mod health;
pub mod images;
pub mod kanban_columns;
pub mod oauth;
pub mod organizations;
pub mod projects;
pub mod repo;
pub mod scratch;
pub mod sessions;
pub mod shared_tasks;
pub mod state_transitions;
pub mod tags;
pub mod task_attempts;
pub mod task_events;
pub mod task_triggers;
pub mod tasks;
pub mod workflow_templates;

pub fn router(deployment: DeploymentImpl) -> IntoMakeService<Router> {
    // Create routers with different middleware layers
    let base_routes = Router::new()
        .route("/health", get(health::health_check))
        .merge(config::router())
        .merge(containers::router(&deployment))
        .merge(projects::router(&deployment))
        .merge(tasks::router(&deployment))
        .merge(task_events::router(&deployment))
        .merge(task_triggers::router(&deployment))
        .merge(shared_tasks::router())
        .merge(task_attempts::router(&deployment))
        .merge(execution_processes::router(&deployment))
        .merge(tags::router(&deployment))
        .merge(oauth::router())
        .merge(organizations::router())
        .merge(filesystem::router())
        .merge(repo::router())
        .merge(events::router(&deployment))
        .merge(approvals::router())
        .merge(scratch::router(&deployment))
        .merge(sessions::router(&deployment))
        .merge(agents::router(&deployment))
        .merge(kanban_columns::router(&deployment))
        .merge(state_transitions::router(&deployment))
        .merge(automation_rules::router(&deployment))
        .merge(boards::router(&deployment))
        .merge(debug_events::router(&deployment))
        .merge(context_artifacts::router(&deployment))
        .merge(workflow_templates::router(&deployment))
        .nest("/images", images::routes())
        .with_state(deployment);

    Router::new()
        .route("/", get(frontend::serve_frontend_root))
        .route("/{*path}", get(frontend::serve_frontend))
        .nest("/api", base_routes)
        .into_make_service()
}

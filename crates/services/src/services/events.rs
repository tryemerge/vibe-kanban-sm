use std::sync::Arc;

use db::DBService;
use tokio::sync::RwLock;
use utils::msg_store::MsgStore;

#[path = "events/patches.rs"]
pub mod patches;
#[path = "events/streams.rs"]
mod streams;
#[path = "events/types.rs"]
pub mod types;

pub use patches::{
    execution_process_patch, project_patch, scratch_patch, task_patch, workspace_patch,
};
pub use types::{EventError, EventPatch, EventPatchInner, HookTables, RecordTypes};

#[derive(Clone)]
pub struct EventService {
    msg_store: Arc<MsgStore>,
    #[allow(dead_code)]
    db: DBService,
    #[allow(dead_code)]
    entry_count: Arc<RwLock<usize>>,
}

impl EventService {
    /// Creates a new EventService
    /// Note: PostgreSQL doesn't support the same preupdate/update hooks as SQLite.
    /// Real-time events will need to be implemented using PostgreSQL LISTEN/NOTIFY.
    pub fn new(db: DBService, msg_store: Arc<MsgStore>, entry_count: Arc<RwLock<usize>>) -> Self {
        tracing::warn!(
            "EventService: PostgreSQL mode - real-time database hooks are not implemented. \
             Consider implementing LISTEN/NOTIFY for real-time updates."
        );
        Self {
            msg_store,
            db,
            entry_count,
        }
    }

    /// Creates a no-op hook for PostgreSQL
    /// SQLite had preupdate and update hooks, but PostgreSQL uses LISTEN/NOTIFY instead.
    /// This is a stub that does nothing - real-time updates will need a different implementation.
    pub fn create_hook(
        _msg_store: Arc<MsgStore>,
        _entry_count: Arc<RwLock<usize>>,
        _db_service: DBService,
    ) -> impl for<'a> Fn(
        &'a mut sqlx::PgConnection,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<(), sqlx::Error>> + Send + 'a>,
    > + Send
    + Sync
    + 'static {
        move |_conn: &mut sqlx::PgConnection| {
            Box::pin(async move {
                // PostgreSQL doesn't support SQLite-style update hooks.
                // Real-time updates should be implemented using:
                // 1. PostgreSQL LISTEN/NOTIFY with triggers
                // 2. Polling
                // 3. Application-level event emission after writes
                Ok(())
            })
        }
    }

    pub fn msg_store(&self) -> &Arc<MsgStore> {
        &self.msg_store
    }
}

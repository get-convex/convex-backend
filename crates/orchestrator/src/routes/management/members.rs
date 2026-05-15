use axum::Router;

use crate::state::OrchestratorState;

pub fn router() -> Router<OrchestratorState> {
    // Member management is handled via the dashboard private API (`profile`,
    // `member_data`). The public Management API does not currently expose
    // member CRUD beyond the team-membership endpoints in `teams.rs`.
    Router::new()
}

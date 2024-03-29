// @generated - do not modify. Modify build.rs instead.
#![allow(clippy::match_single_binding)]
pub mod document_id;
pub mod error_metadata;
pub mod field_path;
pub mod user_identity_attributes;
pub mod backend {
    include!(concat!(env!("OUT_DIR"), "/backend.rs"));
}
pub mod common {
    include!(concat!(env!("OUT_DIR"), "/common.rs"));
}
pub mod convex_actions {
    include!(concat!(env!("OUT_DIR"), "/convex_actions.rs"));
}
pub mod convex_cursor {
    include!(concat!(env!("OUT_DIR"), "/convex_cursor.rs"));
}
pub mod convex_identity {
    include!(concat!(env!("OUT_DIR"), "/convex_identity.rs"));
}
pub mod convex_keys {
    include!(concat!(env!("OUT_DIR"), "/convex_keys.rs"));
}
pub mod convex_query_journal {
    include!(concat!(env!("OUT_DIR"), "/convex_query_journal.rs"));
}
pub mod convex_token {
    include!(concat!(env!("OUT_DIR"), "/convex_token.rs"));
}
pub mod errors {
    include!(concat!(env!("OUT_DIR"), "/errors.rs"));
}
pub mod funrun {
    include!(concat!(env!("OUT_DIR"), "/funrun.rs"));
}
pub mod searchlight {
    include!(concat!(env!("OUT_DIR"), "/searchlight.rs"));
}
pub mod storage {
    include!(concat!(env!("OUT_DIR"), "/storage.rs"));
}
pub mod usage {
    include!(concat!(env!("OUT_DIR"), "/usage.rs"));
}

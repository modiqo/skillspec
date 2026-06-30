mod apply;
mod model;
mod render;
mod store;
mod target;

use std::path::PathBuf;

pub(crate) use apply::apply_to_candidates;
pub use model::*;
pub use render::{
    render_get, render_init, render_list, render_profile_apply, render_profile_clear,
    render_profile_status, render_remove_rule, render_set_profile, render_set_rule, render_show,
};
pub use store::{
    create_schema, get, init, list, profile_apply, profile_clear, profile_status, remove_rule,
    set_profile, set_rule, show,
};

pub(crate) fn normalize_index_path(path: PathBuf) -> PathBuf {
    crate::router::normalize_index_path(path)
}

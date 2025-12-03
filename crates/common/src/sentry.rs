use sentry::Scope;

pub fn set_sentry_tags(scope: &mut Scope) {
    if let Ok(alloc_id) = std::env::var("NOMAD_ALLOC_ID") {
        scope.set_tag("nomad_alloc_id", alloc_id);
    }
    if let Ok(job_name) = std::env::var("NOMAD_JOB_NAME") {
        scope.set_tag("nomad_job_name", job_name);
    }
    if let Ok(group_name) = std::env::var("NOMAD_GROUP_NAME") {
        scope.set_tag("nomad_group_name", group_name);
    }
    if let Ok(task_name) = std::env::var("NOMAD_TASK_NAME") {
        scope.set_tag("nomad_task_name", task_name);
    }
    if let Ok(dc) = std::env::var("NOMAD_DC") {
        scope.set_tag("nomad_dc", dc);
    }
}

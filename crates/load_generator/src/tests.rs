use anyhow::Context;

use crate::Workload;

#[test]
fn test_workloads_parse() -> anyhow::Result<()> {
    let workloads_dir = std::fs::read_dir("workloads")?;
    for entry in workloads_dir {
        let entry = entry?;
        let path = entry.path();
        let workload = std::fs::read_to_string(&path)?;
        let _workload: Workload = serde_json::from_str(&workload)
            .with_context(|| format!("Failed to parse workload from string: {workload}"))?;
    }
    Ok(())
}

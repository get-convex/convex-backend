use std::collections::BTreeMap;

use common::components::ComponentPath;
use thousands::Separable;
use value::TableName;

pub struct TableChange {
    pub added: u64,
    pub deleted: usize,
    pub existing: usize,
    pub unit: &'static str,
    pub is_missing_id_field: bool,
}

pub fn render_table_changes(
    table_changes: BTreeMap<(ComponentPath, TableName), TableChange>,
) -> Vec<String> {
    // Looks like:
    /*
    table    | create  | delete                       |
    ---------------------------------------------------
    _storage | 10      | 11 of 11 files               |
    big      | 100,000 | 100,000 of 100,000 documents |
    messages | 20      | 21 of 21 documents           |
            */
    let mut message_lines = Vec::new();
    let mut parts = vec![(
        "table".to_string(),
        "create".to_string(),
        "delete".to_string(),
    )];
    for (
        (_, table_name),
        TableChange {
            added,
            deleted,
            existing,
            unit,
            is_missing_id_field: _,
        },
    ) in table_changes
    {
        parts.push((
            table_name.to_string(),
            added.separate_with_commas(),
            format!(
                "{} of {}{}",
                deleted.separate_with_commas(),
                existing.separate_with_commas(),
                unit
            ),
        ));
    }
    let part_lengths = (
        parts
            .iter()
            .map(|p| p.0.len())
            .max()
            .expect("should be nonempty"),
        parts
            .iter()
            .map(|p| p.1.len())
            .max()
            .expect("should be nonempty"),
        parts
            .iter()
            .map(|p| p.2.len())
            .max()
            .expect("should be nonempty"),
    );
    for (i, part) in parts.into_iter().enumerate() {
        message_lines.push(format!(
            "{:3$} | {:4$} | {:5$} |",
            part.0, part.1, part.2, part_lengths.0, part_lengths.1, part_lengths.2
        ));
        if i == 0 {
            message_lines.push(format!(
                "{:-<1$}",
                "",
                part_lengths.0 + 3 + part_lengths.1 + 3 + part_lengths.2 + 2
            ));
        }
    }
    message_lines
}

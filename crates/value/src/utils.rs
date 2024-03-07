use std::{
    fmt,
    fmt::Display,
};

use crate::{
    TableMapping,
    TableName,
    TableNumber,
    VirtualTableMapping,
};

fn display_composite<K: Display, V: Display, I: Iterator<Item = (Option<K>, V)>>(
    f: &mut fmt::Formatter,
    enclosing: [&str; 2],
    items: I,
) -> fmt::Result {
    let mut first = true;
    write!(f, "{}", enclosing[0])?;
    for (key, value) in items {
        if !first {
            write!(f, ", ")?;
        }
        if let Some(key) = key {
            write!(f, "{}: ", key)?;
        }
        write!(f, "{}", value)?;
        first = false;
    }
    write!(f, "{}", enclosing[1])
}

/// Format an iterator of `items` with a comma separator and enclosed by
/// `enclosing[0]` and `enclosing[1]`.
pub fn display_sequence<V: Display>(
    f: &mut fmt::Formatter,
    enclosing: [&str; 2],
    items: impl Iterator<Item = V>,
) -> fmt::Result {
    // Since we're passing in `None` for the key type, we need to pass in something
    // for the first type parameter to help type inference out.
    display_composite::<usize, V, _>(f, enclosing, items.map(|v| (None, v)))
}

/// Format an iterator of key-value pairs with a comma separator and enclosed by
/// `enclosing[0]` and `enclosing[1]`.
pub fn display_map<K: Display, V: Display>(
    f: &mut fmt::Formatter,
    enclosing: [&str; 2],
    items: impl Iterator<Item = (K, V)>,
) -> fmt::Result {
    display_composite(f, enclosing, items.map(|(k, v)| (Some(k), v)))
}

// Checks both virtual tables and tables to get the table number to name mapping
pub fn all_tables_number_to_name(
    table_mapping: &TableMapping,
    virtual_table_mapping: &VirtualTableMapping,
) -> impl Fn(TableNumber) -> anyhow::Result<TableName> {
    let table_mapping = table_mapping.clone();
    let virtual_table_mapping = virtual_table_mapping.clone();
    move |number| {
        if let Ok(table_number) = virtual_table_mapping.name(number) {
            return Ok(table_number);
        }
        table_mapping.number_to_name()(number)
    }
}

// Checks both virtual tables and tables to get the table name to number mapping
pub fn all_tables_name_to_number(
    table_mapping: &TableMapping,
    virtual_table_mapping: &VirtualTableMapping,
) -> impl Fn(TableName) -> anyhow::Result<TableNumber> {
    let table_mapping = table_mapping.clone();
    let virtual_table_mapping = virtual_table_mapping.clone();
    move |name| {
        if let Ok(number) = virtual_table_mapping.name_to_number_user_input()(name.clone()) {
            return Ok(number);
        }
        table_mapping.name_to_number_user_input()(name)
    }
}

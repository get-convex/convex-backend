use std::mem;

use crate::{
    assert_obj,
    ConvexObject,
    ConvexValue,
    InternalId,
    ResolvedDocumentId,
    Size,
    TableIdAndTableNumber,
    TableIdentifier,
};

#[test]
fn test_value_size() -> anyhow::Result<()> {
    // Feel free to change this, but it's good to be aware of how large our values
    // are.
    assert_eq!(mem::size_of::<ConvexValue>(), 56);
    assert_eq!(mem::size_of::<ConvexValue>(), 56);

    // Changing the computed size of a value can make stored TableSummary
    // inconsistent, so when changing this you need to also rewrite all
    // TableSummary snapshots.
    let value: ConvexValue = ResolvedDocumentId::new(
        <TableIdAndTableNumber as TableIdentifier>::min(),
        InternalId::MIN,
    )
    .into();
    assert_eq!(value.size(), 33);
    Ok(())
}

#[test]
fn test_object_cmp() -> anyhow::Result<()> {
    // Equal ignoring order.
    let o1: ConvexObject = assert_obj!(
        "chambers" => 36,
        "cuban_linx" => 4,
    );
    let o2: ConvexObject = assert_obj!(
        "cuban_linx" => 4,
        "chambers" => 36,
    );
    assert!(*o1 == *o2);

    // Lexicographic ordering on fields.
    let o1: ConvexObject = assert_obj!(
        "nested" => { "compton" => 187 },
    );
    let o2: ConvexObject = assert_obj!(
        "nested" => { "bompton" => 187 },
    );
    assert!(*o2 < *o1);

    // Ordered on values if same fields.
    let o1: ConvexObject = assert_obj!("_93_til" => 94);
    let o2: ConvexObject = assert_obj!("_93_til" => 95);
    assert!(*o1 < *o2);

    Ok(())
}

#[test]
fn test_shallow_merge() -> anyhow::Result<()> {
    // Overwrite objects with non-objects
    let mut old: ConvexObject = assert_obj!(
        "name" => {
            "first" => "Mr",
            "last" => {
                "a" => "Fanta",
                "b" => "stik",
            },
        },
    );
    let new = assert_obj!(
        "name" => {
            "first" => "Mr",
            "last" => "Fantastik",
        },
    );
    let expected = assert_obj!(
        "name" => {
            "first" => "Mr",
            "last" => "Fantastik",
        },
    );
    old = old.shallow_merge(new)?;
    assert!(*old == *expected);

    // Overwrite non-objects with objects
    let mut old: ConvexObject = assert_obj!(
        "name" => "Mr",
    );
    let new = assert_obj!(
        "name" => {
            "first" => "Mr",
            "surname" => "Fantastik",
        },
    );
    let expected = assert_obj!(
        "name" => {
            "first" => "Mr",
            "surname" => "Fantastik",
        },
    );
    old = old.shallow_merge(new)?;
    assert!(*old == *expected);

    // Don't merge sub-fields
    let mut old: ConvexObject = assert_obj!(
        "name" => {
            "first" => "Mr",
            "last" => "Fantastik",
        },
    );
    let new = assert_obj!(
        "name" => {
            "first" => "Mr",
            "surname" => "Fantastik",
        },
    );
    let expected = assert_obj!(
        "name" => {
            "first" => "Mr",
            "surname" => "Fantastik",
        },
    );
    old = old.shallow_merge(new)?;
    assert!(*old == *expected);

    Ok(())
}

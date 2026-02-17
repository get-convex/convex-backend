//! FlowField resolution, ComputedField expression evaluation, and aggregation.
//!
//! FlowFields are cross-table aggregations resolved at read time.
//! ComputedFields are row-level expressions evaluated from stored + FlowField
//! values.
//!
//! This module provides:
//! - `resolve_document_fields()` — top-level resolver called from async_syscall
//! - `resolve_flow_fields()` — FlowField aggregation via index range scans
//! - `evaluate_expr()` — recursive expression evaluator for ComputedFields
//! - `resolve_computed_fields()` — topological evaluation of computed fields

use std::collections::BTreeMap;

use anyhow::Context;
use common::{
    bootstrap_model::{
        index::IndexConfig,
        schema::SchemaState,
    },
    document::{
        DeveloperDocument,
        ResolvedDocument,
    },
    interval::Interval,
    query::{
        CursorPosition,
        Order,
    },
    runtime::Runtime,
    schemas::{
        default_for_validator,
        FlowFieldAggregation,
        FlowFieldSchema,
    },
    types::{
        IndexName,
        TabletIndexName,
    },
};
use indexing::backend_in_memory_indexes::RangeRequest;
use value::{
    sorting::values_to_bytes,
    ConvexValue,
    FieldName,
    FieldPath,
    TableNamespace,
    TableNumber,
};

use crate::{
    query::IndexRangeResponse,
    Transaction,
};

/// Top-level resolver: given a document and its table name, resolve any
/// FlowFields and ComputedFields defined in the active schema.
///
/// Returns `None` if there are no flow/computed fields to resolve (fast path).
/// Returns `Some(extra_fields)` with the additional fields to merge into the
/// document.
pub async fn resolve_document_fields<RT: Runtime>(
    tx: &mut Transaction<RT>,
    namespace: TableNamespace,
    doc: &DeveloperDocument,
    table_number: TableNumber,
) -> anyhow::Result<Option<BTreeMap<FieldName, ConvexValue>>> {
    // Get the active schema for this namespace.
    let schema = match tx.get_schema_by_state(namespace, SchemaState::Active)? {
        Some((_id, schema)) => schema,
        None => return Ok(None),
    };

    // Look up the table name from the table number.
    let table_name = match tx.table_mapping().namespace(namespace).number_to_name()(table_number) {
        Ok(name) => name,
        Err(_) => return Ok(None),
    };

    // Look up the table definition in the schema.
    let table_def = match schema.tables.get(&table_name) {
        Some(def) => def,
        None => return Ok(None),
    };

    // Fast path: no flow fields and no computed fields.
    if table_def.flow_fields.is_empty() && table_def.computed_fields.is_empty() {
        return Ok(None);
    }

    let mut extra_fields = BTreeMap::new();

    // Step 1: Resolve FlowFields (cross-table aggregations).
    for flow_field in &table_def.flow_fields {
        let value = resolve_single_flow_field(tx, namespace, doc, flow_field).await?;
        let field_name: FieldName = flow_field.field_name.clone().into();
        extra_fields.insert(field_name, value);
    }

    // Step 2: Resolve ComputedFields (expression evaluation).
    // Computed fields can reference stored fields, flow fields, or other
    // computed fields. We evaluate them in declaration order (the user is
    // responsible for declaring dependencies before dependents).
    let doc_value = doc.value().0.clone();
    let mut all_fields: BTreeMap<String, ConvexValue> = BTreeMap::new();

    // Add stored fields.
    let stored: BTreeMap<FieldName, ConvexValue> = doc_value.into();
    for (field_name, value) in &stored {
        all_fields.insert(field_name.to_string(), value.clone());
    }

    // Add flow fields.
    for (field_name, value) in &extra_fields {
        all_fields.insert(field_name.to_string(), value.clone());
    }

    // Evaluate computed fields in order.
    for computed in &table_def.computed_fields {
        let value = match evaluate_expr(&computed.expr, &all_fields) {
            Ok(v) => v,
            Err(_) => default_for_validator(&computed.returns),
        };
        let field_name: FieldName = computed.field_name.clone().into();
        all_fields.insert(field_name.to_string(), value.clone());
        extra_fields.insert(field_name, value);
    }

    if extra_fields.is_empty() {
        Ok(None)
    } else {
        Ok(Some(extra_fields))
    }
}

/// Resolve a single FlowField by scanning the source table's index and
/// aggregating.
async fn resolve_single_flow_field<RT: Runtime>(
    tx: &mut Transaction<RT>,
    namespace: TableNamespace,
    doc: &DeveloperDocument,
    flow_field: &FlowFieldSchema,
) -> anyhow::Result<ConvexValue> {
    // Find the source table's tablet ID.
    let source_tablet_id = match tx
        .table_mapping()
        .namespace(namespace)
        .id_if_exists(&flow_field.source)
    {
        Some(id) => id,
        None => return Ok(default_for_validator(&flow_field.returns)),
    };

    // Find an index on the source table where the first field is the key field.
    // We scan all enabled indexes looking for one with a matching first field.
    let index_registry = tx.index.index_registry();
    let mut matching_index = None;
    for index_doc in index_registry.enabled_indexes_for_table(source_tablet_id) {
        let IndexConfig::Database { spec, .. } = &index_doc.config else {
            continue;
        };
        let indexed_fields: Vec<FieldPath> = spec.fields.clone().into();
        if let Some(first_field) = indexed_fields.first() {
            let field_names = first_field.fields();
            if field_names.len() == 1 && *field_names[0] == *flow_field.key {
                matching_index = Some((
                    TabletIndexName::new(source_tablet_id, index_doc.name.descriptor().clone())?,
                    IndexName::new(
                        flow_field.source.clone(),
                        index_doc.name.descriptor().clone(),
                    )?,
                    spec.fields.clone(),
                ));
                break;
            }
        }
    }

    let (index_name, printable_index_name, indexed_fields) = match matching_index {
        Some(idx) => idx,
        // No suitable index found — return default.
        None => return Ok(default_for_validator(&flow_field.returns)),
    };

    // Build the interval: prefix scan for key == doc._id (as ConvexValue).
    let doc_id_value = ConvexValue::from(doc.id());
    let prefix_bytes = values_to_bytes(&[Some(doc_id_value)]);
    let interval = Interval::prefix(prefix_bytes.into());

    // Execute the range scan, collecting all matching documents.
    let mut all_docs = Vec::new();
    let mut current_interval = interval.clone();

    loop {
        let range_request = RangeRequest {
            index_name: index_name.clone(),
            printable_index_name: printable_index_name.clone(),
            interval: current_interval.clone(),
            order: Order::Asc,
            max_size: 1000,
        };

        let [result] = tx
            .index
            .range_batch(&[&range_request])
            .await
            .try_into()
            .map_err(|_| anyhow::anyhow!("expected single result"))?;

        let IndexRangeResponse { page, cursor } = result?;

        for (_key, doc, _ts) in page {
            all_docs.push(doc);
        }

        match cursor {
            CursorPosition::End => break,
            CursorPosition::After(last_key) => {
                // Continue from after the last key.
                let (_, remaining) = current_interval.split_after(last_key.into(), Order::Asc);
                current_interval = remaining;
            },
        }
    }

    // Record read dependency for subscription invalidation.
    tx.reads
        .record_indexed_directly(index_name, indexed_fields, interval)?;

    // Apply static filters.
    let filtered_docs: Vec<_> = if let Some(filter) = &flow_field.filter {
        all_docs
            .into_iter()
            .filter(|doc| matches_filter(doc, filter))
            .collect()
    } else {
        all_docs
    };

    // Aggregate.
    aggregate(
        &flow_field.aggregation,
        &filtered_docs,
        flow_field.field.as_deref(),
        &flow_field.returns,
    )
}

/// Check if a document matches the static filter conditions.
fn matches_filter(doc: &ResolvedDocument, filter: &serde_json::Value) -> bool {
    let filter_obj = match filter.as_object() {
        Some(obj) => obj,
        None => return true,
    };

    let doc_fields: BTreeMap<FieldName, ConvexValue> = doc.value().0.clone().into();

    for (key, expected_value) in filter_obj {
        // Skip $field references (FlowFilter parameters — Phase 4).
        if let Some(obj) = expected_value.as_object() {
            if obj.contains_key("$field") {
                continue;
            }
        }

        let field_name: FieldName = match key.parse() {
            Ok(f) => f,
            Err(_) => return false,
        };

        let actual = match doc_fields.get(&field_name) {
            Some(v) => v,
            None => return false,
        };

        if !json_value_matches_convex(expected_value, actual) {
            return false;
        }
    }

    true
}

/// Compare a JSON filter value to a ConvexValue.
fn json_value_matches_convex(json: &serde_json::Value, convex: &ConvexValue) -> bool {
    match (json, convex) {
        (serde_json::Value::Bool(b), ConvexValue::Boolean(c)) => b == c,
        (serde_json::Value::Number(n), ConvexValue::Float64(c)) => {
            n.as_f64().map_or(false, |f| f == *c)
        },
        (serde_json::Value::Number(n), ConvexValue::Int64(c)) => {
            n.as_i64().map_or(false, |i| i == *c)
        },
        (serde_json::Value::String(s), ConvexValue::String(c)) => s == c.as_ref(),
        (serde_json::Value::Null, ConvexValue::Null) => true,
        _ => false,
    }
}

/// Perform aggregation over the filtered documents.
fn aggregate(
    agg_type: &FlowFieldAggregation,
    docs: &[ResolvedDocument],
    field: Option<&str>,
    returns: &common::schemas::validator::Validator,
) -> anyhow::Result<ConvexValue> {
    match agg_type {
        FlowFieldAggregation::Count => Ok(ConvexValue::Float64(docs.len() as f64)),

        FlowFieldAggregation::Exist => Ok(ConvexValue::Boolean(!docs.is_empty())),

        FlowFieldAggregation::Sum => {
            let field = field.context("sum aggregation requires a 'field'")?;
            let sum: f64 = docs
                .iter()
                .filter_map(|doc| extract_numeric(doc, field))
                .sum();
            Ok(ConvexValue::Float64(sum))
        },

        FlowFieldAggregation::Avg => {
            let field = field.context("avg aggregation requires a 'field'")?;
            let values: Vec<f64> = docs
                .iter()
                .filter_map(|doc| extract_numeric(doc, field))
                .collect();
            if values.is_empty() {
                Ok(default_for_validator(returns))
            } else {
                let sum: f64 = values.iter().sum();
                Ok(ConvexValue::Float64(sum / values.len() as f64))
            }
        },

        FlowFieldAggregation::Min => {
            let field = field.context("min aggregation requires a 'field'")?;
            let min = docs
                .iter()
                .filter_map(|doc| extract_numeric(doc, field))
                .reduce(f64::min);
            match min {
                Some(v) => Ok(ConvexValue::Float64(v)),
                None => Ok(default_for_validator(returns)),
            }
        },

        FlowFieldAggregation::Max => {
            let field = field.context("max aggregation requires a 'field'")?;
            let max = docs
                .iter()
                .filter_map(|doc| extract_numeric(doc, field))
                .reduce(f64::max);
            match max {
                Some(v) => Ok(ConvexValue::Float64(v)),
                None => Ok(default_for_validator(returns)),
            }
        },

        FlowFieldAggregation::Lookup => {
            // Lookup returns the first matching document's field value, or default.
            let field = field.context("lookup aggregation requires a 'field'")?;
            let value = docs.first().and_then(|doc| {
                let fields: BTreeMap<FieldName, ConvexValue> = doc.value().0.clone().into();
                let field_name: FieldName = field.parse().ok()?;
                fields.get(&field_name).cloned()
            });
            Ok(value.unwrap_or_else(|| default_for_validator(returns)))
        },
    }
}

/// Extract a numeric value from a document's field.
fn extract_numeric(doc: &ResolvedDocument, field: &str) -> Option<f64> {
    let fields: BTreeMap<FieldName, ConvexValue> = doc.value().0.clone().into();
    let field_name: FieldName = field.parse().ok()?;
    match fields.get(&field_name) {
        Some(ConvexValue::Float64(f)) => Some(*f),
        Some(ConvexValue::Int64(i)) => Some(*i as f64),
        _ => None,
    }
}

// ── Expression Evaluator for ComputedFields ──────────────────────────────────

/// Evaluate a ComputedField expression against a set of field values.
///
/// The expression DSL supports:
/// - `"$fieldName"` — field reference (string starting with $)
/// - Literals: number, string, bool, null
/// - `{ "$add": [a, b] }`, `$sub`, `$mul`, `$div` — arithmetic
/// - `{ "$gt": [a, b] }`, `$gte`, `$lt`, `$lte`, `$eq`, `$ne` — comparisons
/// - `{ "$cond": <bool_expr>, "then": <expr>, "else": <expr> }` — conditional
/// - `{ "$concat": [a, b, ...] }` — string concatenation
/// - `{ "$ifNull": [a, b] }` — null coalescing
pub fn evaluate_expr(
    expr: &serde_json::Value,
    fields: &BTreeMap<String, ConvexValue>,
) -> anyhow::Result<ConvexValue> {
    match expr {
        // String — could be a field reference ($fieldName) or a literal string.
        serde_json::Value::String(s) => {
            if let Some(field_name) = s.strip_prefix('$') {
                fields
                    .get(field_name)
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("Field reference ${field_name} not found"))
            } else {
                Ok(ConvexValue::String(
                    s.as_str()
                        .try_into()
                        .context("invalid string in expression")?,
                ))
            }
        },

        // Number literal.
        serde_json::Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                Ok(ConvexValue::Float64(f))
            } else {
                anyhow::bail!("Unsupported number in expression: {n}")
            }
        },

        // Boolean literal.
        serde_json::Value::Bool(b) => Ok(ConvexValue::Boolean(*b)),

        // Null literal.
        serde_json::Value::Null => Ok(ConvexValue::Null),

        // Object — operator expression.
        serde_json::Value::Object(obj) => {
            // Arithmetic operators.
            if let Some(args) = obj.get("$add") {
                return eval_binary_arithmetic(args, fields, |a, b| a + b);
            }
            if let Some(args) = obj.get("$sub") {
                return eval_binary_arithmetic(args, fields, |a, b| a - b);
            }
            if let Some(args) = obj.get("$mul") {
                return eval_binary_arithmetic(args, fields, |a, b| a * b);
            }
            if let Some(args) = obj.get("$div") {
                return eval_binary_arithmetic(args, fields, |a, b| {
                    if b == 0.0 {
                        f64::NAN
                    } else {
                        a / b
                    }
                });
            }

            // Comparison operators.
            if let Some(args) = obj.get("$gt") {
                return eval_comparison(args, fields, |a, b| a > b);
            }
            if let Some(args) = obj.get("$gte") {
                return eval_comparison(args, fields, |a, b| a >= b);
            }
            if let Some(args) = obj.get("$lt") {
                return eval_comparison(args, fields, |a, b| a < b);
            }
            if let Some(args) = obj.get("$lte") {
                return eval_comparison(args, fields, |a, b| a <= b);
            }
            if let Some(args) = obj.get("$eq") {
                return eval_eq(args, fields, false);
            }
            if let Some(args) = obj.get("$ne") {
                return eval_eq(args, fields, true);
            }

            // Conditional: { "$cond": <bool_expr>, "then"|"if": <expr>, "else": <expr> }
            if let Some(cond) = obj.get("$cond") {
                let then_expr = obj
                    .get("then")
                    .or_else(|| obj.get("if"))
                    .context("$cond missing 'then' (or 'if') branch")?;
                let else_expr = obj.get("else").context("$cond missing 'else' branch")?;
                let cond_value = evaluate_expr(cond, fields)?;
                return if is_truthy(&cond_value) {
                    evaluate_expr(then_expr, fields)
                } else {
                    evaluate_expr(else_expr, fields)
                };
            }

            // String concatenation: { "$concat": [a, b, ...] }
            if let Some(args) = obj.get("$concat") {
                let args = args.as_array().context("$concat requires an array")?;
                let mut result = std::string::String::new();
                for arg in args {
                    let val = evaluate_expr(arg, fields)?;
                    result.push_str(&convex_to_string(&val));
                }
                return Ok(ConvexValue::String(
                    result
                        .as_str()
                        .try_into()
                        .context("concat result too large")?,
                ));
            }

            // Null coalescing: { "$ifNull": [a, b] }
            if let Some(args) = obj.get("$ifNull") {
                let args = args.as_array().context("$ifNull requires an array")?;
                anyhow::ensure!(args.len() == 2, "$ifNull requires exactly 2 arguments");
                let first = evaluate_expr(&args[0], fields)?;
                if matches!(first, ConvexValue::Null) {
                    return evaluate_expr(&args[1], fields);
                }
                return Ok(first);
            }

            anyhow::bail!("Unknown expression operator in: {expr}")
        },

        // Array — not a valid top-level expression.
        serde_json::Value::Array(_) => {
            anyhow::bail!("Arrays are not valid expressions (did you mean $concat?)")
        },
    }
}

fn eval_binary_arithmetic(
    args: &serde_json::Value,
    fields: &BTreeMap<String, ConvexValue>,
    op: fn(f64, f64) -> f64,
) -> anyhow::Result<ConvexValue> {
    let args = args
        .as_array()
        .context("arithmetic operator requires an array of 2")?;
    anyhow::ensure!(
        args.len() == 2,
        "arithmetic operator requires exactly 2 arguments"
    );
    let a = to_f64(&evaluate_expr(&args[0], fields)?)?;
    let b = to_f64(&evaluate_expr(&args[1], fields)?)?;
    Ok(ConvexValue::Float64(op(a, b)))
}

fn eval_comparison(
    args: &serde_json::Value,
    fields: &BTreeMap<String, ConvexValue>,
    op: fn(f64, f64) -> bool,
) -> anyhow::Result<ConvexValue> {
    let args = args
        .as_array()
        .context("comparison operator requires an array of 2")?;
    anyhow::ensure!(
        args.len() == 2,
        "comparison operator requires exactly 2 arguments"
    );
    let a = to_f64(&evaluate_expr(&args[0], fields)?)?;
    let b = to_f64(&evaluate_expr(&args[1], fields)?)?;
    Ok(ConvexValue::Boolean(op(a, b)))
}

fn eval_eq(
    args: &serde_json::Value,
    fields: &BTreeMap<String, ConvexValue>,
    negate: bool,
) -> anyhow::Result<ConvexValue> {
    let args = args
        .as_array()
        .context("equality operator requires an array of 2")?;
    anyhow::ensure!(
        args.len() == 2,
        "equality operator requires exactly 2 arguments"
    );
    let a = evaluate_expr(&args[0], fields)?;
    let b = evaluate_expr(&args[1], fields)?;
    let eq = convex_values_equal(&a, &b);
    Ok(ConvexValue::Boolean(if negate { !eq } else { eq }))
}

fn convex_values_equal(a: &ConvexValue, b: &ConvexValue) -> bool {
    match (a, b) {
        (ConvexValue::Null, ConvexValue::Null) => true,
        (ConvexValue::Boolean(a), ConvexValue::Boolean(b)) => a == b,
        (ConvexValue::Float64(a), ConvexValue::Float64(b)) => a == b,
        (ConvexValue::Int64(a), ConvexValue::Int64(b)) => a == b,
        (ConvexValue::Float64(a), ConvexValue::Int64(b)) => *a == *b as f64,
        (ConvexValue::Int64(a), ConvexValue::Float64(b)) => *a as f64 == *b,
        (ConvexValue::String(a), ConvexValue::String(b)) => a == b,
        _ => false,
    }
}

fn to_f64(v: &ConvexValue) -> anyhow::Result<f64> {
    match v {
        ConvexValue::Float64(f) => Ok(*f),
        ConvexValue::Int64(i) => Ok(*i as f64),
        ConvexValue::Null => Ok(0.0),
        other => anyhow::bail!("Expected numeric value, got: {other:?}"),
    }
}

fn is_truthy(v: &ConvexValue) -> bool {
    match v {
        ConvexValue::Boolean(b) => *b,
        ConvexValue::Null => false,
        ConvexValue::Float64(f) => *f != 0.0,
        ConvexValue::Int64(i) => *i != 0,
        ConvexValue::String(s) => !s.is_empty(),
        _ => true,
    }
}

fn convex_to_string(v: &ConvexValue) -> std::string::String {
    match v {
        ConvexValue::String(s) => s.to_string(),
        ConvexValue::Float64(f) => f.to_string(),
        ConvexValue::Int64(i) => i.to_string(),
        ConvexValue::Boolean(b) => b.to_string(),
        ConvexValue::Null => "null".to_string(),
        _ => format!("{v:?}"),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use common::schemas::{
        default_for_validator,
        validator::{
            FieldValidator,
            ObjectValidator,
            Validator,
        },
    };
    use serde_json::json;
    use value::ConvexValue;

    use super::evaluate_expr;

    // ── default_for_validator tests ──────────────────────────────────────

    #[test]
    fn test_default_float64() {
        assert_eq!(
            default_for_validator(&Validator::Float64),
            ConvexValue::Float64(0.0)
        );
    }

    #[test]
    fn test_default_int64() {
        assert_eq!(
            default_for_validator(&Validator::Int64),
            ConvexValue::Int64(0)
        );
    }

    #[test]
    fn test_default_string() {
        assert_eq!(
            default_for_validator(&Validator::String),
            ConvexValue::String("".try_into().unwrap())
        );
    }

    #[test]
    fn test_default_boolean() {
        assert_eq!(
            default_for_validator(&Validator::Boolean),
            ConvexValue::Boolean(false)
        );
    }

    #[test]
    fn test_default_null() {
        assert_eq!(default_for_validator(&Validator::Null), ConvexValue::Null);
    }

    #[test]
    fn test_default_array() {
        let default = default_for_validator(&Validator::Array(Box::new(Validator::Float64)));
        match default {
            ConvexValue::Array(arr) => assert!(arr.is_empty()),
            _ => panic!("Expected empty array"),
        }
    }

    #[test]
    fn test_default_union() {
        // Union defaults to the first variant's default.
        let default = default_for_validator(&Validator::Union(vec![
            Validator::String,
            Validator::Float64,
        ]));
        assert_eq!(default, ConvexValue::String("".try_into().unwrap()));
    }

    #[test]
    fn test_default_any() {
        assert_eq!(default_for_validator(&Validator::Any), ConvexValue::Null);
    }

    // ── Expression evaluator tests ──────────────────────────────────────

    fn fields(pairs: Vec<(&str, ConvexValue)>) -> BTreeMap<String, ConvexValue> {
        pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
    }

    #[test]
    fn test_field_reference() -> anyhow::Result<()> {
        let f = fields(vec![("name", ConvexValue::String("Alice".try_into()?))]);
        let result = evaluate_expr(&json!("$name"), &f)?;
        assert_eq!(result, ConvexValue::String("Alice".try_into()?));
        Ok(())
    }

    #[test]
    fn test_literal_string() -> anyhow::Result<()> {
        let f = fields(vec![]);
        let result = evaluate_expr(&json!("hello"), &f)?;
        assert_eq!(result, ConvexValue::String("hello".try_into()?));
        Ok(())
    }

    #[test]
    fn test_literal_number() -> anyhow::Result<()> {
        let f = fields(vec![]);
        let result = evaluate_expr(&json!(42.0), &f)?;
        assert_eq!(result, ConvexValue::Float64(42.0));
        Ok(())
    }

    #[test]
    fn test_literal_bool() -> anyhow::Result<()> {
        let f = fields(vec![]);
        let result = evaluate_expr(&json!(true), &f)?;
        assert_eq!(result, ConvexValue::Boolean(true));
        Ok(())
    }

    #[test]
    fn test_literal_null() -> anyhow::Result<()> {
        let f = fields(vec![]);
        let result = evaluate_expr(&json!(null), &f)?;
        assert_eq!(result, ConvexValue::Null);
        Ok(())
    }

    #[test]
    fn test_add() -> anyhow::Result<()> {
        let f = fields(vec![
            ("a", ConvexValue::Float64(10.0)),
            ("b", ConvexValue::Float64(20.0)),
        ]);
        let result = evaluate_expr(&json!({"$add": ["$a", "$b"]}), &f)?;
        assert_eq!(result, ConvexValue::Float64(30.0));
        Ok(())
    }

    #[test]
    fn test_sub() -> anyhow::Result<()> {
        let f = fields(vec![
            ("a", ConvexValue::Float64(50.0)),
            ("b", ConvexValue::Float64(20.0)),
        ]);
        let result = evaluate_expr(&json!({"$sub": ["$a", "$b"]}), &f)?;
        assert_eq!(result, ConvexValue::Float64(30.0));
        Ok(())
    }

    #[test]
    fn test_mul() -> anyhow::Result<()> {
        let f = fields(vec![
            ("a", ConvexValue::Float64(5.0)),
            ("b", ConvexValue::Float64(4.0)),
        ]);
        let result = evaluate_expr(&json!({"$mul": ["$a", "$b"]}), &f)?;
        assert_eq!(result, ConvexValue::Float64(20.0));
        Ok(())
    }

    #[test]
    fn test_div() -> anyhow::Result<()> {
        let f = fields(vec![
            ("a", ConvexValue::Float64(20.0)),
            ("b", ConvexValue::Float64(4.0)),
        ]);
        let result = evaluate_expr(&json!({"$div": ["$a", "$b"]}), &f)?;
        assert_eq!(result, ConvexValue::Float64(5.0));
        Ok(())
    }

    #[test]
    fn test_div_by_zero() -> anyhow::Result<()> {
        let f = fields(vec![
            ("a", ConvexValue::Float64(20.0)),
            ("b", ConvexValue::Float64(0.0)),
        ]);
        let result = evaluate_expr(&json!({"$div": ["$a", "$b"]}), &f)?;
        match result {
            ConvexValue::Float64(v) => assert!(v.is_nan()),
            _ => panic!("Expected NaN"),
        }
        Ok(())
    }

    #[test]
    fn test_gt() -> anyhow::Result<()> {
        let f = fields(vec![("x", ConvexValue::Float64(10.0))]);
        assert_eq!(
            evaluate_expr(&json!({"$gt": ["$x", 5]}), &f)?,
            ConvexValue::Boolean(true)
        );
        assert_eq!(
            evaluate_expr(&json!({"$gt": ["$x", 10]}), &f)?,
            ConvexValue::Boolean(false)
        );
        Ok(())
    }

    #[test]
    fn test_gte() -> anyhow::Result<()> {
        let f = fields(vec![("x", ConvexValue::Float64(10.0))]);
        assert_eq!(
            evaluate_expr(&json!({"$gte": ["$x", 10]}), &f)?,
            ConvexValue::Boolean(true)
        );
        assert_eq!(
            evaluate_expr(&json!({"$gte": ["$x", 11]}), &f)?,
            ConvexValue::Boolean(false)
        );
        Ok(())
    }

    #[test]
    fn test_eq() -> anyhow::Result<()> {
        let f = fields(vec![("x", ConvexValue::String("hello".try_into()?))]);
        assert_eq!(
            evaluate_expr(&json!({"$eq": ["$x", "hello"]}), &f)?,
            ConvexValue::Boolean(true)
        );
        assert_eq!(
            evaluate_expr(&json!({"$eq": ["$x", "world"]}), &f)?,
            ConvexValue::Boolean(false)
        );
        Ok(())
    }

    #[test]
    fn test_ne() -> anyhow::Result<()> {
        let f = fields(vec![("x", ConvexValue::Float64(5.0))]);
        assert_eq!(
            evaluate_expr(&json!({"$ne": ["$x", 5]}), &f)?,
            ConvexValue::Boolean(false)
        );
        assert_eq!(
            evaluate_expr(&json!({"$ne": ["$x", 6]}), &f)?,
            ConvexValue::Boolean(true)
        );
        Ok(())
    }

    #[test]
    fn test_cond() -> anyhow::Result<()> {
        let f = fields(vec![("totalSpent", ConvexValue::Float64(1500.0))]);
        let expr = json!({
            "$cond": {"$gt": ["$totalSpent", 1000]},
            "then": "VIP",
            "else": "STANDARD"
        });
        assert_eq!(
            evaluate_expr(&expr, &f)?,
            ConvexValue::String("VIP".try_into()?)
        );

        let f2 = fields(vec![("totalSpent", ConvexValue::Float64(500.0))]);
        assert_eq!(
            evaluate_expr(&expr, &f2)?,
            ConvexValue::String("STANDARD".try_into()?)
        );
        Ok(())
    }

    #[test]
    fn test_concat() -> anyhow::Result<()> {
        let f = fields(vec![
            ("name", ConvexValue::String("Alice".try_into()?)),
            ("tier", ConvexValue::String("VIP".try_into()?)),
        ]);
        let result = evaluate_expr(&json!({"$concat": ["$name", " (", "$tier", ")"]}), &f)?;
        assert_eq!(result, ConvexValue::String("Alice (VIP)".try_into()?));
        Ok(())
    }

    #[test]
    fn test_if_null() -> anyhow::Result<()> {
        let f = fields(vec![
            ("nickname", ConvexValue::Null),
            ("name", ConvexValue::String("Alice".try_into()?)),
        ]);
        let result = evaluate_expr(&json!({"$ifNull": ["$nickname", "$name"]}), &f)?;
        assert_eq!(result, ConvexValue::String("Alice".try_into()?));

        // Non-null first arg.
        let f2 = fields(vec![
            ("nickname", ConvexValue::String("Ali".try_into()?)),
            ("name", ConvexValue::String("Alice".try_into()?)),
        ]);
        let result2 = evaluate_expr(&json!({"$ifNull": ["$nickname", "$name"]}), &f2)?;
        assert_eq!(result2, ConvexValue::String("Ali".try_into()?));
        Ok(())
    }

    #[test]
    fn test_nested_expression() -> anyhow::Result<()> {
        let f = fields(vec![
            ("price", ConvexValue::Float64(100.0)),
            ("tax", ConvexValue::Float64(10.0)),
            ("discount", ConvexValue::Float64(5.0)),
        ]);
        // (price + tax) - discount = 105
        let expr = json!({
            "$sub": [
                {"$add": ["$price", "$tax"]},
                "$discount"
            ]
        });
        assert_eq!(evaluate_expr(&expr, &f)?, ConvexValue::Float64(105.0));
        Ok(())
    }

    #[test]
    fn test_null_in_arithmetic() -> anyhow::Result<()> {
        // Null is treated as 0 in arithmetic.
        let f = fields(vec![("x", ConvexValue::Null)]);
        let result = evaluate_expr(&json!({"$add": ["$x", 5]}), &f)?;
        assert_eq!(result, ConvexValue::Float64(5.0));
        Ok(())
    }

    #[test]
    fn test_int64_in_arithmetic() -> anyhow::Result<()> {
        let f = fields(vec![
            ("a", ConvexValue::Int64(10)),
            ("b", ConvexValue::Float64(2.5)),
        ]);
        let result = evaluate_expr(&json!({"$mul": ["$a", "$b"]}), &f)?;
        assert_eq!(result, ConvexValue::Float64(25.0));
        Ok(())
    }

    #[test]
    fn test_missing_field_reference_errors() {
        let f = fields(vec![]);
        let result = evaluate_expr(&json!("$nonexistent"), &f);
        assert!(result.is_err());
    }

    #[test]
    fn test_concat_with_numbers() -> anyhow::Result<()> {
        let f = fields(vec![("count", ConvexValue::Float64(42.0))]);
        let result = evaluate_expr(&json!({"$concat": ["count: ", "$count"]}), &f)?;
        assert_eq!(result, ConvexValue::String("count: 42".try_into()?));
        Ok(())
    }
}

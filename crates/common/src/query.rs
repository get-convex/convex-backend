//! Types for querying a database.

use std::{
    collections::BTreeMap,
    fmt::Display,
    io::Write,
    ops::Bound,
};

use errors::ErrorMetadata;
use itertools::{
    Either,
    Itertools,
};
use pb::{
    convex_cursor::IndexKey as IndexKeyProto,
    funrun::cursor::Position as PositionProto,
};
use serde::Serialize;
use serde_json::Value as JsonValue;
use sha2::{
    Digest,
    Sha256,
};
use value::{
    heap_size::HeapSize,
    id_v6::VirtualTableNumberMap,
    utils::display_sequence,
    val,
    ConvexObject,
    ConvexValue,
    DeveloperDocumentId,
    TableId,
    TableIdAndTableNumber,
};

use crate::{
    bootstrap_model::index::database_index::IndexedFields,
    document::ID_FIELD_PATH,
    index::IndexKeyBytes,
    interval::{
        BinaryKey,
        End,
        Interval,
        Start,
    },
    paths::FieldPath,
    types::{
        GenericIndexName,
        IndexName,
        MaybeValue,
        TableName,
    },
    value::{
        sha256::Sha256 as CommonSha256,
        values_to_bytes,
    },
};
/// Serialized cursor representation for sending to clients.
pub type SerializedCursor = String;

/// A hash of the query that's included in cursors.
pub type QueryFingerprint = Vec<u8>;

/// A `CursorPosition` is a position within query results used to implement
/// `paginate()`.
#[derive(Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum CursorPosition {
    After(IndexKeyBytes),
    End,
}

impl HeapSize for CursorPosition {
    fn heap_size(&self) -> usize {
        match self {
            CursorPosition::After(bytes) => bytes.heap_size(),
            CursorPosition::End => 0,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct Cursor {
    pub position: CursorPosition,

    /// Hashed representation of the query this cursor refers to.
    pub query_fingerprint: QueryFingerprint,
}

impl From<Cursor> for pb::funrun::Cursor {
    fn from(
        Cursor {
            position,
            query_fingerprint,
        }: Cursor,
    ) -> Self {
        let position = match position {
            CursorPosition::End => PositionProto::End(()),
            CursorPosition::After(ref key) => PositionProto::After(IndexKeyProto {
                values: key.clone().0,
            }),
        };
        Self {
            position: Some(position),
            query_fingerprint: Some(query_fingerprint),
        }
    }
}

impl TryFrom<pb::funrun::Cursor> for Cursor {
    type Error = anyhow::Error;

    fn try_from(
        pb::funrun::Cursor {
            position,
            query_fingerprint,
        }: pb::funrun::Cursor,
    ) -> anyhow::Result<Self> {
        let position = position.ok_or_else(|| anyhow::anyhow!("Cursor is missing position"))?;
        let position = match position {
            pb::funrun::cursor::Position::After(index_key) => {
                CursorPosition::After(IndexKeyBytes(index_key.values))
            },
            pb::funrun::cursor::Position::End(()) => CursorPosition::End,
        };
        Ok(Self {
            position,
            query_fingerprint: query_fingerprint
                .ok_or_else(|| anyhow::anyhow!("Missing query_fingerprint"))?,
        })
    }
}

impl HeapSize for Cursor {
    fn heap_size(&self) -> usize {
        self.position.heap_size() + self.query_fingerprint.heap_size()
    }
}

#[derive(Clone, Copy, Eq, Hash, PartialEq, Debug)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
/// The order to scan a range.
pub enum Order {
    /// Ascending order, e.g. 1, 2, 3.
    Asc,
    /// Descending order, e.g. 3, 2, 1.
    Desc,
}

impl Order {
    /// Apply an ordering to an iterator, reversing it if `self == Order::Desc`.
    pub fn apply<T>(
        &self,
        iter: impl DoubleEndedIterator<Item = T>,
    ) -> impl DoubleEndedIterator<Item = T> {
        match self {
            Order::Asc => Either::Left(iter),
            Order::Desc => Either::Right(iter.rev()),
        }
    }
}

/// A range of an index to query.
#[derive(Clone, Debug, PartialEq)]
pub struct IndexRange {
    /// The index being scanned.
    pub index_name: IndexName,

    /// The range of the index to scan.
    /// These expressions must be in index order, with the `Eq` expressions
    /// preceding the others (which matches what an index can actually do
    /// efficiently).
    pub range: Vec<IndexRangeExpression>,
    /// The order to scan in.
    pub order: Order,
}

impl IndexRange {
    pub fn compile(
        self,
        indexed_fields: IndexedFields,
        virtual_table_number_map: Option<VirtualTableNumberMap>,
    ) -> anyhow::Result<Interval> {
        let index_name = self.index_name.clone();
        let SplitIndexRange {
            equalities,
            inequality,
        } = self.split()?.map_values(|field, v| {
            if field == &*ID_FIELD_PATH {
                map_id_value_to_tablet(v, virtual_table_number_map)
            } else {
                Ok(v)
            }
        })?;

        // Check that some permutation of the equality field paths + the (optional)
        // inequality field path is a prefix of the indexed paths.
        let index_rank: BTreeMap<_, _> = indexed_fields
            .iter_with_id()
            .enumerate()
            .map(|(i, field_name)| (field_name, i))
            .collect();
        anyhow::ensure!(
            index_rank.len() == indexed_fields.iter_with_id().count(),
            "{index_name} has duplicate fields?"
        );

        let mut equalities: Vec<_> = equalities
            .into_iter()
            .map(|(field, value)| -> anyhow::Result<_> {
                if let Some(rank) = index_rank.get(&field) {
                    Ok((field, value, *rank))
                } else {
                    anyhow::bail!(field_not_in_index_error(
                        &index_name,
                        &field,
                        &indexed_fields
                    ))
                }
            })
            .try_collect()?;
        equalities.sort_by_key(|(_, _, rank)| *rank);

        if let Some(ref inequality) = inequality
            && !index_rank.contains_key(&inequality.field_path)
        {
            anyhow::bail!(field_not_in_index_error(
                &index_name,
                &inequality.field_path,
                &indexed_fields,
            ))
        }

        let used_paths: Vec<_> = equalities
            .iter()
            .map(|(field_path, ..)| field_path.clone())
            .chain(
                inequality
                    .as_ref()
                    .map(|inequality| inequality.field_path.clone()),
            )
            .collect();

        let query_fields = QueryFields(used_paths.clone());

        let mut fields_iter = indexed_fields.iter_with_id();
        for field_path in used_paths {
            let matching_field = fields_iter.next().ok_or_else(|| {
                invalid_index_range(&index_name, &indexed_fields, &query_fields, &field_path)
            })?;
            if field_path != *matching_field {
                anyhow::bail!(invalid_index_range(
                    &index_name,
                    &indexed_fields,
                    &query_fields,
                    &field_path,
                ));
            }
        }

        // Now that we know the index expression is compatible with the index, turn it
        // into an interval.
        let prefix: Vec<_> = equalities.into_iter().map(|(_, v, _)| v.0).collect();
        let result = if let Some(inequality) = inequality {
            let start = match inequality.start {
                Bound::Unbounded => BinaryKey::from(values_to_bytes(&prefix)),
                Bound::Included(value) => {
                    let mut bound = prefix.clone();
                    bound.push(Some(value));
                    BinaryKey::from(values_to_bytes(&bound))
                },
                Bound::Excluded(value) => {
                    let mut bound = prefix.clone();
                    bound.push(Some(value));
                    BinaryKey::from(values_to_bytes(&bound))
                        .increment()
                        .ok_or_else(|| anyhow::anyhow!("{bound:?} should have an increment"))?
                },
            };
            let end = match inequality.end {
                Bound::Unbounded => End::after_prefix(&BinaryKey::from(values_to_bytes(&prefix))),
                Bound::Included(value) => {
                    let mut bound = prefix;
                    bound.push(Some(value));
                    End::after_prefix(&BinaryKey::from(values_to_bytes(&bound)))
                },
                Bound::Excluded(value) => {
                    let mut bound = prefix;
                    bound.push(Some(value));
                    End::Excluded(BinaryKey::from(values_to_bytes(&bound)))
                },
            };
            Interval {
                start: Start::Included(start),
                end,
            }
        } else {
            let prefix_key = BinaryKey::from(values_to_bytes(&prefix));
            Interval::prefix(prefix_key)
        };
        Ok(result)
    }

    fn split(self) -> anyhow::Result<SplitIndexRange> {
        let mut equalities = BTreeMap::new();

        let mut inequality_field_path: Option<FieldPath> = None;
        let mut inequality_start = Bound::Unbounded;
        let mut inequality_end = Bound::Unbounded;

        for expr in self.range {
            let (field_path, value, is_less, is_equals) = match expr {
                IndexRangeExpression::Eq(field_path, value) => {
                    if let Some(old_value) = equalities.insert(field_path.clone(), value) {
                        let error =
                            already_defined_bound_error("equality", &field_path, "==", &old_value);
                        anyhow::bail!(error);
                    }
                    continue;
                },
                IndexRangeExpression::Gt(field_path, value) => (field_path, value, false, false),
                IndexRangeExpression::Gte(field_path, value) => (field_path, value, false, true),
                IndexRangeExpression::Lt(field_path, value) => (field_path, value, true, false),
                IndexRangeExpression::Lte(field_path, value) => (field_path, value, true, true),
            };

            // Check that we're defining the bound for the first time.
            let destination = if is_less {
                &mut inequality_end
            } else {
                &mut inequality_start
            };
            if *destination != Bound::Unbounded {
                let bound_type = if is_less { "upper" } else { "lower" };
                let comparator = match (is_less, is_equals) {
                    (false, false) => ">",
                    (false, true) => ">=",
                    (true, false) => "<",
                    (true, true) => "<=",
                };
                let error =
                    already_defined_bound_error(bound_type, &field_path, comparator, &value.into());
                anyhow::bail!(error);
            }
            // Check that all of the inequalities are for the same field path.
            if let Some(ref first_path) = inequality_field_path {
                if first_path != &field_path {
                    anyhow::bail!(bounds_on_multiple_fields_error(
                        &self.index_name,
                        first_path,
                        &field_path,
                    ));
                }
            };
            inequality_field_path = Some(field_path);

            *destination = if is_equals {
                Bound::Included(value)
            } else {
                Bound::Excluded(value)
            };
        }
        if let Some(ref inequality_field_path) = inequality_field_path {
            if let Some(equality_value) = equalities.get(inequality_field_path) {
                let error = already_defined_bound_error(
                    "inequality",
                    inequality_field_path,
                    "==",
                    equality_value,
                );
                anyhow::bail!(error);
            }
        }

        let inequality = inequality_field_path.map(|field_path| IndexInequality {
            field_path,
            start: inequality_start,
            end: inequality_end,
        });
        let result = SplitIndexRange {
            equalities,
            inequality,
        };
        Ok(result)
    }
}

// Helper struct for the intermediate state of `IndexRange::compile`. We want to
// turn a user-specified list of index range expressions into a set of equality
// constraints and then a single inequality at the end.
struct SplitIndexRange {
    equalities: BTreeMap<FieldPath, MaybeValue>,
    inequality: Option<IndexInequality>,
}

impl SplitIndexRange {
    pub fn map_values(
        self,
        f: impl Fn(&FieldPath, ConvexValue) -> anyhow::Result<ConvexValue>,
    ) -> anyhow::Result<SplitIndexRange> {
        let equalities = self
            .equalities
            .into_iter()
            .map(|(field, value)| {
                let new_value = match value.0 {
                    Some(value) => MaybeValue(Some(f(&field, value)?)),
                    None => MaybeValue(None),
                };
                anyhow::Ok((field, new_value))
            })
            .try_collect()?;
        let inequality = self
            .inequality
            .map(|inequality| {
                let start = match inequality.start {
                    Bound::Unbounded => Bound::Unbounded,
                    Bound::Included(value) => Bound::Included(f(&inequality.field_path, value)?),
                    Bound::Excluded(value) => Bound::Excluded(f(&inequality.field_path, value)?),
                };
                let end = match inequality.end {
                    Bound::Unbounded => Bound::Unbounded,
                    Bound::Included(value) => Bound::Included(f(&inequality.field_path, value)?),
                    Bound::Excluded(value) => Bound::Excluded(f(&inequality.field_path, value)?),
                };
                anyhow::Ok(IndexInequality {
                    field_path: inequality.field_path,
                    start,
                    end,
                })
            })
            .transpose()?;
        Ok(SplitIndexRange {
            equalities,
            inequality,
        })
    }
}

struct IndexInequality {
    field_path: FieldPath,
    start: Bound<ConvexValue>,
    end: Bound<ConvexValue>,
}

/// A wrapper to pretty print the fields in a query for error messages.
#[derive(Clone, Debug)]
struct QueryFields(Vec<FieldPath>);

impl Display for QueryFields {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        display_sequence(f, ["[", "]"], self.0.iter())
    }
}

fn map_id_value_to_tablet(
    value: ConvexValue,
    virtual_table_number_map: Option<VirtualTableNumberMap>,
) -> anyhow::Result<ConvexValue> {
    let val = match (&value, virtual_table_number_map) {
        (ConvexValue::String(id), Some(virtual_table_number_map)) => {
            let mapped =
                DeveloperDocumentId::map_string_between_table_numbers(id, virtual_table_number_map);
            val!(mapped)
        },
        _ => value,
    };
    Ok(val)
}

fn already_defined_bound_error(
    bound_type: &str,
    field_path: &FieldPath,
    comparator: &str,
    value: &MaybeValue,
) -> ErrorMetadata {
    ErrorMetadata::bad_request(
        "AlreadyDefinedBound",
        format!(
            "Already defined {bound_type} bound in index range. Can't add {field_path:?} \
             {comparator} {value}."
        ),
    )
}

fn bounds_on_multiple_fields_error(
    index_name: &IndexName,
    first_field_path: &FieldPath,
    second_field_path: &FieldPath,
) -> ErrorMetadata {
    ErrorMetadata::bad_request(
        "BoundsOnMultipleFields",
        format!("Upper and lower bounds in `range` can only be applied to a single index \
    field. This query against index {index_name} attempted to set a range \
    bound on both {first_field_path:?} and {second_field_path:?}. Consider using \
    `filter` instead. See https://docs.convex.dev/using/indexes for more info."),
    )
}

fn invalid_index_range(
    name: &IndexName,
    indexed_fields: &IndexedFields,
    query_fields: &QueryFields,
    field_path: &FieldPath,
) -> ErrorMetadata {
    ErrorMetadata::bad_request(
        "InvalidIndexRange",
        format!(
            "Tried to query index {name} but the query didn't use the index fields in order.\n\
             \
             Index fields: {indexed_fields}\n\
             Query fields: {query_fields}\n\
             First incorrect field: {field_path}\n\
             \
             For more information see https://docs.convex.dev/using/indexes."
        ),
    )
}

fn field_not_in_index_error(
    index_name: &IndexName,
    field_path: &FieldPath,
    indexed_fields: &IndexedFields,
) -> ErrorMetadata {
    ErrorMetadata::bad_request(
        "FieldNotInIndex",
        format!("The index range included a comparison with {field_path:?}, but {index_name} with fields {indexed_fields} doesn't index this field. For more information see https://docs.convex.dev/using/indexes."),
    )
}

/// A restriction on the range of an index to query.
/// These are expressed as operators on the index fields.
#[derive(Clone, Debug, PartialEq)]
pub enum IndexRangeExpression {
    Eq(FieldPath, MaybeValue),
    Gt(FieldPath, ConvexValue),
    Gte(FieldPath, ConvexValue),
    Lt(FieldPath, ConvexValue),
    Lte(FieldPath, ConvexValue),
}

/// A table to scan
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct FullTableScan {
    /// The name of the table to scan
    pub table_name: TableName,

    /// The order to scan in.
    pub order: Order,
}

/// Version of full-text search to use
#[derive(Debug, Copy, Clone, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum SearchVersion {
    V1,
    /// Prototype, experimental, don't use in production!
    V2,
}

/// A query against a search index.
///
/// Results are returned in relevancy order based on how well they match
/// the search filter.
#[derive(Clone, Debug, PartialEq)]
pub struct Search {
    /// The search index being queried.
    pub index_name: IndexName,
    pub table: TableName,

    /// The filters to apply within the search index.
    ///
    /// This must include exactly one `Search` expression against the
    /// index's `searchField` and any number of `Eq` expressions comparing
    /// the index's `filterFields`.
    pub filters: Vec<SearchFilterExpression>,
}

impl Search {
    pub fn to_internal(
        self,
        f: &impl Fn(TableName) -> anyhow::Result<TableIdAndTableNumber>,
    ) -> anyhow::Result<InternalSearch> {
        Ok(InternalSearch {
            index_name: self.index_name.to_resolved(f)?.into(),
            table_name: self.table,
            filters: self
                .filters
                .into_iter()
                .map(|f| f.to_internal())
                .collect::<anyhow::Result<Vec<InternalSearchFilterExpression>>>()?,
        })
    }
}

/// While `Search` is constructed and used at the query layer using TableNames,
/// `InternalSearch` is used within transaction and searchlight and uses
/// TableIds.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct InternalSearch {
    /// The search index being queried.
    pub index_name: GenericIndexName<TableId>,
    pub table_name: TableName,

    /// The filters to apply within the search index.
    ///
    /// This must include exactly one `Search` expression against the
    /// index's `searchField` and any number of `Eq` expressions comparing
    /// the index's `filterFields`.
    pub filters: Vec<InternalSearchFilterExpression>,
}

impl InternalSearch {
    pub fn printable_index_name(&self) -> anyhow::Result<IndexName> {
        IndexName::new(
            self.table_name.clone(),
            self.index_name.descriptor().clone(),
        )
    }
}

/// Filter field values under this size are stored as bytes. Otherwise
/// we hash them down to 32 bytes.
const MAX_FILTER_FIELD_LENGTH: usize = 32;
const UNDEFINED_TAG: u8 = 0x1;

pub fn search_value_to_bytes(value: Option<&ConvexValue>) -> Vec<u8> {
    let sort_key = match value {
        Some(value) => value.sort_key(),
        None => vec![UNDEFINED_TAG],
    };
    if sort_key.len() < MAX_FILTER_FIELD_LENGTH {
        sort_key
    } else {
        let hashed_value = CommonSha256::hash(&sort_key);
        Vec::<u8>::from(*hashed_value)
    }
}

/// Filters to apply while querying a search index.
#[derive(Clone, Debug, PartialEq)]
pub enum SearchFilterExpression {
    Search(FieldPath, String),
    Eq(FieldPath, Option<ConvexValue>),
}

/// Filters to apply while querying a search index.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum InternalSearchFilterExpression {
    Search(FieldPath, String),
    Eq(FieldPath, Vec<u8>),
}

impl SearchFilterExpression {
    pub fn to_internal(self) -> anyhow::Result<InternalSearchFilterExpression> {
        let expression = match self {
            Self::Search(field, s) => InternalSearchFilterExpression::Search(field, s),
            Self::Eq(field, v) => {
                InternalSearchFilterExpression::Eq(field, search_value_to_bytes(v.as_ref()))
            },
        };
        Ok(expression)
    }
}

/// The first step of any query is a QuerySource. This defines how the initial
/// row set should be read out of the database, before applying any operators.
#[derive(Clone, Debug, PartialEq)]
pub enum QuerySource {
    /// Scan the entirety of the given table.
    FullTableScan(FullTableScan),
    /// Scan a range of an index.
    IndexRange(IndexRange),
    /// Perform a full text search.
    Search(Search),
}

/// An Expression evaluates to a Value.
///
/// If you add a new expression type, don't forget to add it to the proptest
/// strategy below!
#[derive(Eq, PartialEq, Clone, Debug)]
pub enum Expression {
    /// `l == r`
    Eq(Box<Expression>, Box<Expression>),
    /// `l != r`
    Neq(Box<Expression>, Box<Expression>),
    /// `l < r`
    Lt(Box<Expression>, Box<Expression>),
    /// `l <= r`
    Lte(Box<Expression>, Box<Expression>),
    /// `l > r`
    Gt(Box<Expression>, Box<Expression>),
    /// `l >= r`
    Gte(Box<Expression>, Box<Expression>),
    /// `l + r`
    Add(Box<Expression>, Box<Expression>),
    /// `l - r`
    Sub(Box<Expression>, Box<Expression>),
    /// `l * r`
    Mul(Box<Expression>, Box<Expression>),
    /// `l / r`
    Div(Box<Expression>, Box<Expression>),
    /// `l % r`
    Mod(Box<Expression>, Box<Expression>),
    /// `-x`
    Neg(Box<Expression>),
    /// `a && b && ...`
    And(Vec<Expression>),
    /// `a || b || ...`
    Or(Vec<Expression>),
    /// `!x`
    Not(Box<Expression>),
    /// Evaluates to the named field on the environment Value.
    Field(FieldPath),
    /// A literal value.
    Literal(MaybeValue),
}

#[cfg(any(test, feature = "testing"))]
mod proptest {
    use proptest::prelude::*;
    use value::ConvexValue;

    use super::{
        Expression,
        IndexRange,
        MaybeValue,
        Query,
        QuerySource,
        Search,
    };
    use crate::{
        paths::FieldPath,
        query::{
            FullTableScan,
            IndexRangeExpression,
            Order,
            QueryOperator,
            SearchFilterExpression,
        },
        types::IndexName,
    };

    impl Arbitrary for IndexRangeExpression {
        type Parameters = ();

        type Strategy = impl Strategy<Value = IndexRangeExpression>;

        fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
            prop_oneof![
                any::<(FieldPath, MaybeValue)>()
                    .prop_map(|(field_path, v)| IndexRangeExpression::Eq(field_path, v)),
                any::<(FieldPath, ConvexValue)>()
                    .prop_map(|(field_path, v)| IndexRangeExpression::Gt(field_path, v)),
                any::<(FieldPath, ConvexValue)>()
                    .prop_map(|(field_path, v)| IndexRangeExpression::Gte(field_path, v)),
                any::<(FieldPath, ConvexValue)>()
                    .prop_map(|(field_path, v)| IndexRangeExpression::Lt(field_path, v)),
                any::<(FieldPath, ConvexValue)>()
                    .prop_map(|(field_path, v)| IndexRangeExpression::Lte(field_path, v)),
            ]
        }
    }

    impl Arbitrary for SearchFilterExpression {
        type Parameters = ();

        type Strategy = impl Strategy<Value = SearchFilterExpression>;

        fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
            prop_oneof![
                any::<(FieldPath, String)>()
                    .prop_map(|(field_path, s)| SearchFilterExpression::Search(field_path, s)),
                any::<(FieldPath, Option<ConvexValue>)>()
                    .prop_map(|(field_path, v)| SearchFilterExpression::Eq(field_path, v)),
            ]
        }
    }

    impl Arbitrary for Expression {
        type Parameters = ();

        type Strategy = impl Strategy<Value = Expression>;

        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            let leaf = prop_oneof![
                any::<FieldPath>().prop_map(Expression::Field),
                any::<Option<ConvexValue>>().prop_map(|v| Expression::Literal(MaybeValue(v))),
            ];
            leaf.prop_recursive(
                4,  // 4 levels deep
                10, // Shoot for max 8 nodes
                4,  // Up to 4 items per collection
                |inner| {
                    // Separate helper based on the arguments to this type of expression.
                    let unary = |constructor: fn(Box<Expression>) -> Expression| {
                        inner
                            .clone()
                            .prop_map(move |expr| constructor(Box::new(expr)))
                    };

                    let binary =
                        |constructor: fn(Box<Expression>, Box<Expression>) -> Expression| {
                            (inner.clone(), inner.clone()).prop_map(move |(left, right)| {
                                constructor(Box::new(left), Box::new(right))
                            })
                        };
                    let variadic = |constructor: fn(Vec<Expression>) -> Expression| {
                        prop::collection::vec(inner.clone(), 0..4).prop_map(constructor)
                    };
                    prop_oneof![
                        binary(Expression::Eq),
                        binary(Expression::Neq),
                        binary(Expression::Lt),
                        binary(Expression::Lte),
                        binary(Expression::Gt),
                        binary(Expression::Gte),
                        binary(Expression::Add),
                        binary(Expression::Sub),
                        binary(Expression::Mul),
                        binary(Expression::Div),
                        binary(Expression::Mod),
                        unary(Expression::Neg),
                        variadic(Expression::And),
                        variadic(Expression::Or),
                        unary(Expression::Not),
                    ]
                },
            )
        }
    }

    impl Arbitrary for IndexRange {
        type Parameters = ();

        type Strategy = impl Strategy<Value = IndexRange>;

        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            use proptest::prelude::*;
            (
                prop::collection::vec(any::<IndexRangeExpression>(), 0..4),
                any::<(IndexName, Order)>(),
            )
                .prop_map(|(range, (index_name, order))| IndexRange {
                    range,
                    index_name,
                    order,
                })
        }
    }

    impl Arbitrary for Search {
        type Parameters = ();

        type Strategy = impl Strategy<Value = Search>;

        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            use proptest::prelude::*;
            (
                prop::collection::vec(any::<SearchFilterExpression>(), 0..4),
                any::<IndexName>(),
            )
                .prop_map(|(search_filter_expressions, index_name)| Search {
                    table: index_name.table().clone(),
                    index_name,
                    filters: search_filter_expressions,
                })
        }
    }

    impl Arbitrary for QuerySource {
        type Parameters = ();

        type Strategy = impl Strategy<Value = QuerySource>;

        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            use proptest::prelude::*;
            prop_oneof![
                any::<FullTableScan>().prop_map(QuerySource::FullTableScan),
                any::<IndexRange>().prop_map(QuerySource::IndexRange),
                any::<Search>().prop_map(QuerySource::Search),
            ]
        }
    }

    impl Arbitrary for QueryOperator {
        type Parameters = ();

        type Strategy = impl Strategy<Value = QueryOperator>;

        fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
            prop_oneof![
                any::<Expression>().prop_map(QueryOperator::Filter),
                any::<usize>().prop_map(QueryOperator::Limit)
            ]
        }
    }

    impl Arbitrary for Query {
        type Parameters = ();

        type Strategy = impl Strategy<Value = Query>;

        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            use proptest::prelude::*;
            (
                any::<QuerySource>(),
                prop::collection::vec(any::<QueryOperator>(), 0..4),
            )
                .prop_map(|(source, operators)| Query { source, operators })
        }
    }
}

fn binary_arithmetic<I, F>(
    name: &'static str,
    environ: &ConvexObject,
    l_expr: &Expression,
    r_expr: &Expression,
    do_ints: I,
    do_floats: F,
) -> anyhow::Result<ConvexValue>
where
    I: FnOnce(i64, i64) -> i64,
    F: FnOnce(f64, f64) -> f64,
{
    let l = l_expr.eval(environ)?;
    let r = r_expr.eval(environ)?;

    let result = match (&l.0, &r.0) {
        (Some(ConvexValue::Int64(l)), Some(ConvexValue::Int64(r))) => {
            val!(do_ints(*l, *r))
        },
        (Some(ConvexValue::Float64(l)), Some(ConvexValue::Float64(r))) => {
            val!(do_floats(*l, *r))
        },
        (..) => {
            anyhow::bail!(ErrorMetadata::bad_request(
                "EvalError",
                format!(
                    "Cannot {name} {l} (type {}) and {r} (type {})",
                    l.type_name(),
                    r.type_name()
                ),
            ))
        },
    };
    Ok(result)
}

impl Expression {
    /// Evaluate the expression and return the result. Expression::Fields are
    /// evaluated on `environ`.
    pub fn eval(&self, environ: &ConvexObject) -> anyhow::Result<MaybeValue> {
        // Convert input value into a value that compares with ==, !=, >, <, etc. in
        // the same order as they would be compared in an index.
        let comparable_value = |v: MaybeValue| v.0;
        let result = match self {
            // Field expressions and literals are the two places where `undefined` values
            // originate. Until we migrate our index keys, field expressions use the old behavior
            // that maps missing fields to `Value::Null`.
            Expression::Field(field_path) => {
                return Ok(MaybeValue(environ.get_path(field_path).cloned()));
            },
            Expression::Literal(v) => return Ok(v.clone()),

            // Comparison operations need to operate on `ConvexValue`, not `Value`, so they match
            // the ordering in our index keys, which store table IDs.
            Expression::Eq(l_expr, r_expr) => {
                let l = comparable_value(l_expr.eval(environ)?);
                let r = comparable_value(r_expr.eval(environ)?);
                ConvexValue::from(l == r)
            },
            Expression::Neq(l_expr, r_expr) => {
                let l = comparable_value(l_expr.eval(environ)?);
                let r = comparable_value(r_expr.eval(environ)?);
                ConvexValue::from(l != r)
            },
            Expression::Lt(l_expr, r_expr) => {
                let l = comparable_value(l_expr.eval(environ)?);
                let r = comparable_value(r_expr.eval(environ)?);
                ConvexValue::from(l < r)
            },
            Expression::Lte(l_expr, r_expr) => {
                let l = comparable_value(l_expr.eval(environ)?);
                let r = comparable_value(r_expr.eval(environ)?);
                ConvexValue::from(l <= r)
            },
            Expression::Gt(l_expr, r_expr) => {
                let l = comparable_value(l_expr.eval(environ)?);
                let r = comparable_value(r_expr.eval(environ)?);
                ConvexValue::from(l > r)
            },
            Expression::Gte(l_expr, r_expr) => {
                let l = comparable_value(l_expr.eval(environ)?);
                let r = comparable_value(r_expr.eval(environ)?);
                ConvexValue::from(l >= r)
            },
            // Arithmetic operations only work on Int64 and Float64, so we don't have to worry about
            // mapping those from table names to table IDs.
            Expression::Add(l_expr, r_expr) => {
                binary_arithmetic("add", environ, l_expr, r_expr, |l, r| l + r, |l, r| l + r)?
            },
            Expression::Sub(l_expr, r_expr) => binary_arithmetic(
                "subtract",
                environ,
                l_expr,
                r_expr,
                |l, r| l - r,
                |l, r| l - r,
            )?,
            Expression::Mul(l_expr, r_expr) => binary_arithmetic(
                "multiply",
                environ,
                l_expr,
                r_expr,
                |l, r| l * r,
                |l, r| l * r,
            )?,
            Expression::Div(l_expr, r_expr) => binary_arithmetic(
                "divide",
                environ,
                l_expr,
                r_expr,
                |l, r| l / r,
                |l, r| l / r,
            )?,
            Expression::Mod(l_expr, r_expr) => {
                binary_arithmetic("mod", environ, l_expr, r_expr, |l, r| l % r, |l, r| l % r)?
            },
            Expression::Neg(x_expr) => {
                let x = x_expr.eval(environ)?;
                match &x.0 {
                    Some(ConvexValue::Int64(x)) => ConvexValue::from(-*x),
                    Some(ConvexValue::Float64(x)) => ConvexValue::from(-*x),
                    _ => anyhow::bail!(ErrorMetadata::bad_request(
                        "EvalError",
                        format!("Cannot negate {x} (type {})", x.type_name()),
                    )),
                }
            },
            // Similarly, boolean operations only work on booleans, which don't contain table IDs.
            Expression::And(vs) => {
                for v in vs {
                    if !v.eval(environ)?.into_boolean()? {
                        return Ok(ConvexValue::from(false).into());
                    }
                }
                ConvexValue::from(true)
            },
            Expression::Or(vs) => {
                for v in vs {
                    if v.eval(environ)?.into_boolean()? {
                        return Ok(ConvexValue::from(true).into());
                    }
                }
                ConvexValue::from(false)
            },
            Expression::Not(x_expr) => ConvexValue::from(!x_expr.eval(environ)?.into_boolean()?),
        };
        Ok(result.into())
    }

    /// Shorthand for "field == literal".
    pub fn field_eq_literal(field: FieldPath, literal: ConvexValue) -> Self {
        Expression::Eq(
            Box::new(Expression::Field(field)),
            Box::new(Expression::Literal(literal.into())),
        )
    }

    /// Helper for creating an `And` variant.
    pub fn and(left: Expression, right: Expression) -> Self {
        Expression::And(vec![left, right])
    }
}

/// Queries are lazy iterations, QueryOperators take and produce a stream of
/// Values.
#[derive(Clone, Debug, PartialEq)]
pub enum QueryOperator {
    /// Return only the values for which this expression returns true.
    Filter(Expression),
    /// Return the first n results.
    Limit(usize),
}

/// A query, represented as a source and a chain of operators to apply as a lazy
/// iteration.
#[derive(Clone, Debug, PartialEq)]
pub struct Query {
    /// The original source to fetch values from the database.
    pub source: QuerySource,
    /// The list of operators to apply in order.
    pub operators: Vec<QueryOperator>,
}

impl Query {
    /// Create a query starting with a table scan as the query source.
    pub fn full_table_scan(table_name: TableName, order: Order) -> Self {
        Self {
            source: QuerySource::FullTableScan(FullTableScan { table_name, order }),
            operators: vec![],
        }
    }

    /// Create a query starting with an index range as the query source.
    pub fn index_range(index_range: IndexRange) -> Self {
        Self {
            source: QuerySource::IndexRange(index_range),
            operators: vec![],
        }
    }

    pub fn get(table_name: TableName, id: DeveloperDocumentId) -> Self {
        Self::index_range(IndexRange {
            index_name: IndexName::by_id(table_name),
            range: vec![IndexRangeExpression::Eq(
                ID_FIELD_PATH.clone(),
                MaybeValue(Some(ConvexValue::from(id))),
            )],
            order: Order::Asc,
        })
    }

    pub fn search(search: Search) -> Self {
        Self {
            source: QuerySource::Search(search),
            operators: vec![],
        }
    }

    /// Add a filter predicate to a query.
    pub fn filter(mut self, expression: Expression) -> Self {
        self.operators.push(QueryOperator::Filter(expression));
        self
    }

    pub fn limit(mut self, limit: usize) -> Self {
        self.operators.push(QueryOperator::Limit(limit));
        self
    }

    pub fn fingerprint(&self, indexed_fields: &IndexedFields) -> anyhow::Result<QueryFingerprint> {
        #[derive(Serialize)]
        struct QueryFingerprintJson {
            query: JsonValue,
            indexed_fields: Vec<String>,
        }
        let fingerprint_json = QueryFingerprintJson {
            query: JsonValue::try_from(self.clone())?,
            indexed_fields: indexed_fields
                .iter()
                .map(|field| String::from(field.clone()))
                .collect(),
        };

        // Hash a JSON object of our query plus its indexed fields so the fingerprint
        // changes if any of these change.
        let vec = serde_json::to_vec(&fingerprint_json)?;
        let mut hasher = Sha256::new();
        hasher.write_all(&vec)?;
        Ok(hasher.finalize().to_vec())
    }
}

#[cfg(test)]
mod tests {

    use proptest::prelude::*;
    use sync_types::testing::assert_roundtrips;
    use value::{
        val,
        ConvexValue,
    };

    use super::{
        Expression,
        Order,
        Query,
    };
    use crate::{
        assert_obj,
        bootstrap_model::index::database_index::IndexedFields,
        maybe_val,
        query::{
            Cursor,
            IndexRange,
            IndexRangeExpression,
            MaybeValue,
        },
    };
    #[test]
    fn test_expr_eval() -> anyhow::Result<()> {
        fn test_case(expr: Expression, expected: ConvexValue) -> anyhow::Result<()> {
            let environ = assert_obj!(
                "email" => "bw@convex.dev",
                "salary" => 5,
            );
            assert_eq!(expr.eval(&environ)?, MaybeValue::from(expected));
            Ok(())
        }

        test_case(
            Expression::Eq(
                Box::new(Expression::Literal(maybe_val!("foo"))),
                Box::new(Expression::Literal(maybe_val!("foo"))),
            ),
            val!(true),
        )?;
        test_case(
            Expression::Lt(
                Box::new(Expression::Field("salary".parse()?)),
                Box::new(Expression::Literal(maybe_val!(6))),
            ),
            val!(true),
        )?;
        test_case(
            Expression::Gt(
                Box::new(Expression::Field("level".parse()?)),
                Box::new(Expression::Literal(maybe_val!(2))),
            ),
            ConvexValue::from(false),
        )?;
        test_case(
            Expression::Neq(
                Box::new(Expression::Field("level".parse()?)),
                Box::new(Expression::Literal(maybe_val!(2))),
            ),
            // 2 is indeed not equal to null, even though it may be surprising to get a null value
            // back when you were looking for values that are not 2
            val!(true),
        )?;
        test_case(
            Expression::Or(vec![
                Expression::Gte(
                    Box::new(Expression::Field("level".parse()?)),
                    Box::new(Expression::Literal(maybe_val!(2))),
                ),
                Expression::Lt(
                    Box::new(Expression::Field("level".parse()?)),
                    Box::new(Expression::Literal(maybe_val!(2))),
                ),
            ]),
            // Our total ordering on `Value` allows comparing values of different types.
            val!(true),
        )?;
        test_case(
            Expression::Lt(
                Box::new(Expression::Field("salary".parse()?)),
                Box::new(Expression::Literal(maybe_val!(4))),
            ),
            val!(false),
        )?;
        test_case(
            Expression::Gt(
                Box::new(Expression::Field("salary".parse()?)),
                Box::new(Expression::Literal(maybe_val!(4))),
            ),
            val!(true),
        )?;
        test_case(
            // -6
            Expression::Div(
                // -18
                Box::new(Expression::Neg(
                    // 15 + 3 = 18
                    Box::new(Expression::Add(
                        // 3 * 5 = 15
                        Box::new(Expression::Mul(
                            Box::new(Expression::Literal(maybe_val!(3))),
                            Box::new(Expression::Literal(maybe_val!(5))),
                        )),
                        // 5 - 2 = 3
                        Box::new(Expression::Sub(
                            Box::new(Expression::Literal(maybe_val!(5))),
                            // 11 % 3 = 2
                            Box::new(Expression::Mod(
                                Box::new(Expression::Literal(maybe_val!(11))),
                                Box::new(Expression::Literal(maybe_val!(3))),
                            )),
                        )),
                    )),
                )),
                Box::new(Expression::Literal(maybe_val!(3))),
            ),
            val!(-6),
        )?;
        test_case(
            Expression::Not(Box::new(Expression::Literal(maybe_val!(true)))),
            val!(false),
        )?;
        for i in 0..8 {
            let a = (i & 1) != 0;
            let b = (i & 2) != 0;
            let c = (i & 4) != 0;

            test_case(
                Expression::And(vec![
                    Expression::Literal(maybe_val!(a)),
                    Expression::Literal(maybe_val!(b)),
                    Expression::Literal(maybe_val!(c)),
                ]),
                val!(a && b && c),
            )?;
        }
        for i in 0..8 {
            let a = (i & 1) != 0;
            let b = (i & 2) != 0;
            let c = (i & 4) != 0;

            test_case(
                Expression::Or(vec![
                    Expression::Literal(maybe_val!(a)),
                    Expression::Literal(maybe_val!(b)),
                    Expression::Literal(maybe_val!(c)),
                ]),
                val!(a || b || c),
            )?;
        }

        Ok(())
    }

    #[test]
    fn test_eval_undefined() -> anyhow::Result<()> {
        let environ = assert_obj!(
            "email" => "alpastor@cvx.is",
            "salary" => 5,
        );
        let expr = Expression::Field("name".parse()?);
        assert_eq!(expr.eval(&environ)?, maybe_val!(undefined));

        // Check missing fields equal undefined.
        let expr = Expression::Eq(
            Box::new(Expression::Field("name".parse()?)),
            Box::new(Expression::Literal(maybe_val!(undefined))),
        );
        assert!(expr.eval(&environ)?.into_boolean()?);

        // Check missing fields do not equal null.
        let expr = Expression::Eq(
            Box::new(Expression::Field("name".parse()?)),
            Box::new(Expression::Literal(MaybeValue(Some(ConvexValue::Null)))),
        );
        assert!(!expr.eval(&environ)?.into_boolean()?);

        // Check that nonexistent fields sort before everything else.
        let expr = Expression::Lt(
            Box::new(Expression::Field("name".parse()?)),
            Box::new(Expression::Literal(ConvexValue::Null.into())),
        );
        assert!(expr.eval(&environ)?.into_boolean()?);
        Ok(())
    }

    #[test]
    fn test_query_fingerprint_stability() -> anyhow::Result<()> {
        /*
         * Do not change these hashes!
         *
         * Our pagination code relies on these query fingerprints being stable to
         * check if a cursor is for this query. If our code changes such that
         * our query fingerprints change, users will get pagination errors.
         */

        // Basic full table scan.
        let indexed_fields = IndexedFields::creation_time();
        assert_eq!(
            Query::full_table_scan("MyTable".parse()?, Order::Asc).fingerprint(&indexed_fields)?,
            [
                45, 23, 15, 220, 143, 20, 24, 88, 163, 88, 155, 120, 148, 124, 127, 151, 49, 27,
                45, 248, 63, 108, 127, 47, 211, 64, 13, 50, 103, 138, 80, 215
            ]
        );

        // Complex full table scan.
        let indexed_fields = IndexedFields::creation_time();
        assert_eq!(
            Query::full_table_scan("MyTable".parse()?, Order::Desc)
                .filter(Expression::Eq(
                    Box::new(Expression::Field("channel".parse()?)),
                    Box::new(Expression::Literal(maybe_val!("#general")))
                ))
                .limit(10)
                .fingerprint(&indexed_fields)?,
            vec![
                233, 109, 19, 167, 42, 182, 84, 206, 253, 212, 63, 102, 251, 238, 171, 251, 103, 7,
                15, 237, 13, 236, 235, 161, 87, 58, 96, 81, 138, 157, 30, 194
            ],
        );

        // Indexed query.
        let indexed_fields = vec!["channel".parse()?].try_into()?;
        assert_eq!(
            Query::index_range(IndexRange {
                index_name: "MyTable.my_index".parse()?,
                range: vec![IndexRangeExpression::Eq(
                    "channel".parse()?,
                    maybe_val!("#general")
                )],
                order: Order::Desc
            })
            .fingerprint(&indexed_fields)?,
            vec![
                95, 29, 74, 125, 185, 179, 152, 46, 196, 122, 164, 117, 79, 97, 116, 222, 88, 148,
                238, 241, 117, 13, 129, 67, 108, 84, 35, 89, 100, 65, 114, 114
            ],
        );
        Ok(())
    }

    proptest! {
            #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
        )]
        #[test]
        fn proptest_cursor_serialization(v in any::<Cursor>()) {
            assert_roundtrips::<Cursor, pb::funrun::Cursor>(v);
        }
    }
}

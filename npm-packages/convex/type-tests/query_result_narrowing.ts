import type { GenericTableInfo } from "../src/server/data_model.js";
import type { QueryInitializer } from "../src/server/query.js";
import type {
  IndexRangeBuilder,
  ResultDocumentFromIndexRange,
} from "../src/server/index_range_builder.js";
import type { Expression } from "../src/server/filter_builder.js";
import type { GenericId } from "../src/values/index.js";

type Equal<Left, Right> =
  (<T>() => T extends Left ? 1 : 2) extends <T>() => T extends Right ? 1 : 2
    ? true
    : false;
type Expect<T extends true> = T;
type AwaitedArrayElement<T> =
  Awaited<T> extends Array<infer Element> ? Element : never;
type AwaitedPaginationElement<T> =
  Awaited<T> extends { page: Array<infer Element> } ? Element : never;
type AsyncIterableElement<T> =
  T extends AsyncIterable<infer Element> ? Element : never;
type ExpressionValue<T> = T extends Expression<infer Value> ? Value : never;

type FlashcardVisibility = "private" | "unlisted" | "public";

type FlashcardSet = {
  _id: GenericId<"flashcardSets">;
  _creationTime: number;
  kind?: "deck" | "folder";
  metadata: {
    visibility: FlashcardVisibility;
  };
  profile: {
    topic: "biology" | "math";
  };
  tags: string[];
  targetId: GenericId<"users"> | GenericId<"teams">;
  title: string;
  visibility: FlashcardVisibility;
};

type FlashcardSetsTableInfo = {
  document: FlashcardSet;
  fieldPaths:
    | "_creationTime"
    | "_id"
    | "kind"
    | "metadata"
    | "metadata.visibility"
    | "profile"
    | "profile.topic"
    | "tags"
    | "targetId"
    | "title"
    | "visibility";
  indexes: {
    by_id: ["_id"];
    by_creation_time: ["_creationTime"];
    by_kind: ["kind", "_creationTime"];
    by_metadata_visibility: ["metadata.visibility", "_creationTime"];
    by_profile: ["profile", "_creationTime"];
    by_tags: ["tags", "_creationTime"];
    by_title: ["title", "_creationTime"];
    by_visibility_and_createdAt: ["visibility", "_creationTime"];
    by_visibility_and_target: ["visibility", "targetId", "_creationTime"];
  };
  searchIndexes: {
    search_title: {
      searchField: "title";
      filterFields: "kind" | "targetId" | "visibility";
    };
  };
  vectorIndexes: {};
};

type PublicFlashcardSet = {
  _id: GenericId<"unionFlashcardSets">;
  _creationTime: number;
  kind: "publicSet";
  title: string;
  visibility: "public";
};

type PrivateFlashcardSet = {
  _id: GenericId<"unionFlashcardSets">;
  _creationTime: number;
  kind: "privateSet";
  ownerId: GenericId<"users">;
  title: string;
  visibility: "private";
};

type UnionFlashcardSetsTableInfo = {
  document: PublicFlashcardSet | PrivateFlashcardSet;
  fieldPaths:
    | "_creationTime"
    | "_id"
    | "kind"
    | "ownerId"
    | "title"
    | "visibility";
  indexes: {
    by_id: ["_id"];
    by_creation_time: ["_creationTime"];
    by_kind: ["kind", "_creationTime"];
  };
  searchIndexes: {};
  vectorIndexes: {};
};

type TestDataModel = {
  flashcardSets: FlashcardSetsTableInfo;
  unionFlashcardSets: UnionFlashcardSetsTableInfo;
};

type AssertTableInfoConstraints = [
  Expect<FlashcardSetsTableInfo extends GenericTableInfo ? true : false>,
  Expect<UnionFlashcardSetsTableInfo extends GenericTableInfo ? true : false>,
];

declare const flashcardSets: QueryInitializer<TestDataModel["flashcardSets"]>;
declare const unionFlashcardSets: QueryInitializer<
  TestDataModel["unionFlashcardSets"]
>;
declare const visibilityVariable: FlashcardVisibility;
declare const titleVariable: string;
declare const userId: GenericId<"users">;
declare const indexRangeBuilder: IndexRangeBuilder<
  FlashcardSet,
  ["visibility", "_creationTime"]
>;

const directRange = indexRangeBuilder.eq("visibility", "public");
type DirectRangeDoc = ResultDocumentFromIndexRange<typeof directRange>;
type DirectRangeNarrows = Expect<Equal<DirectRangeDoc["visibility"], "public">>;

const publicDocs = flashcardSets
  .withIndex("by_visibility_and_createdAt", (q) =>
    q.eq("visibility", "public"),
  )
  .take(10);
type PublicDoc = AwaitedArrayElement<typeof publicDocs>;
type PublicVisibilityNarrows = Expect<Equal<PublicDoc["visibility"], "public">>;

const orderedPublicDocs = flashcardSets
  .withIndex("by_visibility_and_createdAt", (q) =>
    q.eq("visibility", "public"),
  )
  .order("desc")
  .take(10);
type OrderedPublicDoc = AwaitedArrayElement<typeof orderedPublicDocs>;
type OrderedPublicVisibilityNarrows = Expect<
  Equal<OrderedPublicDoc["visibility"], "public">
>;

const filteredPublicDocs = flashcardSets
  .withIndex("by_visibility_and_createdAt", (q) =>
    q.eq("visibility", "public"),
  )
  .filter((q) => {
    const visibilityField = q.field("visibility");
    type FilterVisibilityField = ExpressionValue<typeof visibilityField>;
    type FilterVisibilityFieldNarrows = Expect<
      Equal<FilterVisibilityField, "public">
    >;
    return q.eq(visibilityField, "public");
  })
  .take(10);
type FilteredPublicDoc = AwaitedArrayElement<typeof filteredPublicDocs>;
type FilteredPublicVisibilityNarrows = Expect<
  Equal<FilteredPublicDoc["visibility"], "public">
>;

const boundedPublicDocs = flashcardSets
  .withIndex("by_visibility_and_createdAt", (q) =>
    q.eq("visibility", "public").lt("_creationTime", Date.now()),
  )
  .take(10);
type BoundedPublicDoc = AwaitedArrayElement<typeof boundedPublicDocs>;
type BoundedPublicVisibilityNarrows = Expect<
  Equal<BoundedPublicDoc["visibility"], "public">
>;

const firstPublicDoc = flashcardSets
  .withIndex("by_visibility_and_createdAt", (q) =>
    q.eq("visibility", "public"),
  )
  .first();
type FirstPublicDoc = NonNullable<Awaited<typeof firstPublicDoc>>;
type FirstPublicVisibilityNarrows = Expect<
  Equal<FirstPublicDoc["visibility"], "public">
>;

const uniquePublicDoc = flashcardSets
  .withIndex("by_visibility_and_createdAt", (q) =>
    q.eq("visibility", "public"),
  )
  .unique();
type UniquePublicDoc = NonNullable<Awaited<typeof uniquePublicDoc>>;
type UniquePublicVisibilityNarrows = Expect<
  Equal<UniquePublicDoc["visibility"], "public">
>;

const paginatedPublicDocs = flashcardSets
  .withIndex("by_visibility_and_createdAt", (q) =>
    q.eq("visibility", "public"),
  )
  .paginate({ numItems: 10, cursor: null });
type PaginatedPublicDoc = AwaitedPaginationElement<typeof paginatedPublicDocs>;
type PaginatedPublicVisibilityNarrows = Expect<
  Equal<PaginatedPublicDoc["visibility"], "public">
>;

const iterablePublicDocs = flashcardSets.withIndex(
  "by_visibility_and_createdAt",
  (q) => q.eq("visibility", "public"),
);
type IterablePublicDoc = AsyncIterableElement<typeof iterablePublicDocs>;
type IterablePublicVisibilityNarrows = Expect<
  Equal<IterablePublicDoc["visibility"], "public">
>;

const chainedDocs = flashcardSets
  .withIndex("by_visibility_and_target", (q) =>
    q.eq("visibility", "public").eq("targetId", userId),
  )
  .collect();
type ChainedDoc = AwaitedArrayElement<typeof chainedDocs>;
type ChainedVisibilityNarrows = Expect<
  Equal<ChainedDoc["visibility"], "public">
>;
type ChainedIdBrandNarrows = Expect<
  Equal<ChainedDoc["targetId"], GenericId<"users">>
>;

const searchDocs = flashcardSets
  .withSearchIndex("search_title", (q) =>
    q.search("title", "biology").eq("visibility", "public"),
  )
  .take(10);
type SearchDoc = AwaitedArrayElement<typeof searchDocs>;
type SearchVisibilityNarrows = Expect<Equal<SearchDoc["visibility"], "public">>;

const chainedSearchDocs = flashcardSets
  .withSearchIndex("search_title", (q) =>
    q
      .search("title", "biology")
      .eq("visibility", "public")
      .eq("targetId", userId),
  )
  .take(10);
type ChainedSearchDoc = AwaitedArrayElement<typeof chainedSearchDocs>;
type ChainedSearchVisibilityNarrows = Expect<
  Equal<ChainedSearchDoc["visibility"], "public">
>;
type ChainedSearchIdBrandNarrows = Expect<
  Equal<ChainedSearchDoc["targetId"], GenericId<"users">>
>;

const titleLiteralDocs = flashcardSets
  .withIndex("by_title", (q) => q.eq("title", "Biology 101"))
  .take(10);
type TitleLiteralDoc = AwaitedArrayElement<typeof titleLiteralDocs>;
type TitleLiteralNarrows = Expect<
  Equal<TitleLiteralDoc["title"], "Biology 101">
>;

const titleVariableDocs = flashcardSets
  .withIndex("by_title", (q) => q.eq("title", titleVariable))
  .take(10);
type TitleVariableDoc = AwaitedArrayElement<typeof titleVariableDocs>;
type TitleVariableDoesNotNarrow = Expect<
  Equal<TitleVariableDoc["title"], string>
>;

const visibilityVariableDocs = flashcardSets
  .withIndex("by_visibility_and_createdAt", (q) =>
    q.eq("visibility", visibilityVariable),
  )
  .take(10);
type VisibilityVariableDoc = AwaitedArrayElement<typeof visibilityVariableDocs>;
type VisibilityVariableDoesNotOverNarrow = Expect<
  Equal<VisibilityVariableDoc["visibility"], FlashcardVisibility>
>;

const noEqualityDocs = flashcardSets
  .withIndex("by_visibility_and_createdAt")
  .take(10);
type NoEqualityDoc = AwaitedArrayElement<typeof noEqualityDocs>;
type NoEqualityDoesNotNarrow = Expect<
  Equal<NoEqualityDoc["visibility"], FlashcardVisibility>
>;

const identityIndexRangeDocs = flashcardSets
  .withIndex("by_visibility_and_createdAt", (q) => q)
  .take(10);
type IdentityIndexRangeDoc = AwaitedArrayElement<typeof identityIndexRangeDocs>;
type IdentityIndexRangeDoesNotNarrow = Expect<
  Equal<IdentityIndexRangeDoc["visibility"], FlashcardVisibility>
>;

const nestedFieldDocs = flashcardSets
  .withIndex("by_metadata_visibility", (q) =>
    q.eq("metadata.visibility", "public"),
  )
  .take(10);
type NestedFieldDoc = AwaitedArrayElement<typeof nestedFieldDocs>;
type NestedFieldDoesNotNarrow = Expect<
  Equal<NestedFieldDoc["metadata"]["visibility"], FlashcardVisibility>
>;

const objectEqualityDocs = flashcardSets
  .withIndex("by_profile", (q) => q.eq("profile", { topic: "biology" }))
  .take(10);
type ObjectEqualityDoc = AwaitedArrayElement<typeof objectEqualityDocs>;
type ObjectEqualityDoesNotNarrow = Expect<
  Equal<ObjectEqualityDoc["profile"]["topic"], "biology" | "math">
>;

const arrayEqualityDocs = flashcardSets
  .withIndex("by_tags", (q) => q.eq("tags", ["biology"]))
  .take(10);
type ArrayEqualityDoc = AwaitedArrayElement<typeof arrayEqualityDocs>;
type ArrayEqualityDoesNotNarrow = Expect<
  Equal<ArrayEqualityDoc["tags"], string[]>
>;

const optionalFieldDocs = flashcardSets
  .withIndex("by_kind", (q) => q.eq("kind", "deck"))
  .take(10);
type OptionalFieldDoc = AwaitedArrayElement<typeof optionalFieldDocs>;
type OptionalFieldNarrows = Expect<Equal<OptionalFieldDoc["kind"], "deck">>;

const unionDocs = unionFlashcardSets
  .withIndex("by_kind", (q) => q.eq("kind", "publicSet"))
  .collect();
type UnionDoc = AwaitedArrayElement<typeof unionDocs>;
type UnionKindNarrows = Expect<Equal<UnionDoc["kind"], "publicSet">>;
type UnionVisibilityNarrows = Expect<Equal<UnionDoc["visibility"], "public">>;
type UnionBranchNarrows = Expect<
  Equal<"ownerId" extends keyof UnionDoc ? true : false, false>
>;

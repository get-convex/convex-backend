import { GenericDocument } from "convex/server";
import { JSONValue } from "convex/values";
import isEqual from "lodash/isEqual";
import {
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import {
  DatabaseIndexFilterClause,
  Filter,
  FilterExpression,
  SearchIndexFilterClause,
} from "system-udfs/convex/_system/frontend/lib/filters";
import { DeploymentInfoContext } from "@common/lib/deploymentContext";
import {
  DatabaseIndexDef,
  EMPTY_FILTERS,
  IndexDef,
  IndexForField,
  NextIndexedField,
  SearchIndexDef,
  SortOption,
  addIndexedClause,
  addScanClause,
  addSearchFilterClause,
  clearSearchIndex,
  currentDatabaseIndex,
  defaultIndexedClause,
  defaultScanClause,
  defaultSearchFilterValue,
  enabledIndexClauses,
  enabledSearchClauses,
  findIndexDef,
  indexesForField,
  isRangeClause,
  isSearchFilter,
  newClauseId,
  nextIndexedFields,
  nextSearchFilterFields,
  normalizeFilters,
  removeIndexedClause,
  removeScanClause,
  removeSearchFilterClause,
  setDatabaseIndex,
  setIndexedClause,
  setOrder as setOrderInModel,
  setScanClause,
  setSearchFilterClause,
  setSearchIndex,
  setSearchText,
  sortByField as sortByFieldInModel,
  sortOptionForField,
  switchIndexAndAddClause,
} from "./filterModel";

// The representation of a filter clause for presentation.
export type FilterItem =
  | {
      key: string;
      kind: "indexed";
      position: number;
      field: string;
      clause: DatabaseIndexFilterClause;
    }
  | { key: string; kind: "scan"; position: number; clause: Filter }
  | { key: string; kind: "search"; field: string; search: string }
  | {
      key: string;
      kind: "searchFilter";
      position: number;
      clause: SearchIndexFilterClause;
    };

export type FilterItemUpdate =
  | { kind: "indexed"; clause: DatabaseIndexFilterClause }
  | { kind: "scan"; clause: Filter }
  | { kind: "search"; search: string }
  | { kind: "searchFilter"; value: JSONValue | undefined };

export type FilterActions = ReturnType<typeof useFilterActions>;

const scanKey = (clause: Filter, position: number) =>
  `scan/${clause.id ?? position}`;

// Projects the FilterExpression data model into FilterItem
function buildFilterItems(
  defs: IndexDef[],
  expr: FilterExpression,
): FilterItem[] {
  const items: FilterItem[] = [];
  if (isSearchFilter(expr.index)) {
    const def = findIndexDef(defs, expr.index.name);
    items.push({
      key: "search",
      kind: "search",
      field: def?.kind === "search" ? def.searchField : expr.index.name,
      search: expr.index.search,
    });
    expr.index.clauses
      .filter((c) => c.enabled)
      .forEach((clause, position) =>
        items.push({
          key: `searchFilter/${position}`,
          kind: "searchFilter",
          position,
          clause,
        }),
      );
  } else {
    const index = currentDatabaseIndex(defs, expr);
    const used = enabledIndexClauses(expr);
    used.forEach((clause, position) =>
      items.push({
        key: `indexed/${position}`,
        kind: "indexed",
        position,
        field: index.fields[position] ?? "?",
        clause,
      }),
    );
  }
  expr.clauses.forEach((clause, position) =>
    items.push({
      key: scanKey(clause, position),
      kind: "scan",
      position,
      clause,
    }),
  );
  return items;
}

// How long to wait after the last keystroke in a value editor before
// running the query with the new value.
const VALUE_APPLY_DEBOUNCE_MS = 400;

// Owns the draft/applied split for the filter bar and turns user actions
// (add the index's next field, add a scan, remove an item, sort by a
// column, etc) into valid filter expressions. Typed values are debounced so
// each keystroke doesn't run a query.
export function useFilterActions({
  filters,
  draftFilters,
  setDraftFilters,
  applyFilters,
  indexDefs,
  defaultDocument,
}: {
  filters: FilterExpression | undefined;
  draftFilters: FilterExpression | undefined;
  setDraftFilters(next: FilterExpression | undefined): void;
  applyFilters(next: FilterExpression): Promise<void> | void;
  indexDefs: IndexDef[];
  defaultDocument: GenericDocument;
}) {
  const { useLogDeploymentEvent } = useContext(DeploymentInfoContext);
  const log = useLogDeploymentEvent();

  const applied = useMemo(() => filters ?? EMPTY_FILTERS, [filters]);
  const shown = useMemo(() => draftFilters ?? applied, [draftFilters, applied]);

  const apply = useCallback(
    (next: FilterExpression) => {
      setDraftFilters(next);
      void applyFilters(normalizeFilters(next));
    },
    [applyFilters, setDraftFilters],
  );

  const [openItemKey, setOpenItemKey] = useState<string | null>(null);
  const [errors, setErrors] = useState<Record<string, string | undefined>>({});
  // Mirrors `errors` for the debounced apply, which runs outside a render.
  const errorsRef = useRef(errors);
  errorsRef.current = errors;
  const pendingApply = useRef<{
    timer: ReturnType<typeof setTimeout>;
    next: FilterExpression;
  } | null>(null);
  useEffect(
    () => () => {
      if (pendingApply.current) clearTimeout(pendingApply.current.timer);
    },
    [],
  );

  const items = useMemo(
    () => buildFilterItems(indexDefs, shown),
    [indexDefs, shown],
  );
  const hasInvalid = items.some((c) => errors[c.key]);

  const setItemError = useCallback((key: string, messages: string[]) => {
    setErrors((prev) =>
      prev[key] === messages[0] ? prev : { ...prev, [key]: messages[0] },
    );
  }, []);

  const hasErrorsFor = useCallback(
    (expr: FilterExpression) =>
      buildFilterItems(indexDefs, expr).some((c) => errorsRef.current[c.key]),
    [indexDefs],
  );

  const scheduleApply = useCallback(
    (next: FilterExpression) => {
      if (pendingApply.current) clearTimeout(pendingApply.current.timer);
      const timer = setTimeout(() => {
        pendingApply.current = null;
        if (!hasErrorsFor(next)) apply(next);
      }, VALUE_APPLY_DEBOUNCE_MS);
      pendingApply.current = { timer, next };
    },
    [apply, hasErrorsFor],
  );

  const flushPendingApply = useCallback(() => {
    if (!pendingApply.current) return;
    clearTimeout(pendingApply.current.timer);
    const { next } = pendingApply.current;
    pendingApply.current = null;
    if (!hasErrorsFor(next)) apply(next);
  }, [apply, hasErrorsFor]);

  // Structural edits go live right away unless an item still holds an
  // unparseable value, in which case they stay in the draft.
  const commitLive = useCallback(
    (next: FilterExpression) => {
      if (hasInvalid) {
        setDraftFilters(next);
      } else {
        apply(next);
      }
    },
    [apply, hasInvalid, setDraftFilters],
  );

  const indexedNext: NextIndexedField[] = useMemo(
    () => nextIndexedFields(indexDefs, shown),
    [indexDefs, shown],
  );
  const searchFilterNext = useMemo(
    () => nextSearchFilterFields(indexDefs, shown),
    [indexDefs, shown],
  );
  const searchActive = isSearchFilter(shown.index);
  const used = enabledIndexClauses(shown);
  const lastIsRange = used.length > 0 && isRangeClause(used[used.length - 1]);
  const currentIndex = currentDatabaseIndex(indexDefs, shown);
  const nextFields = searchActive
    ? searchFilterNext
    : indexedNext.map((n) => n.field);

  // Adds the index's next clause on `field`, seeded from a sample document
  // so results appear right away, and opens its editor. Only the clause each
  // index kind takes differs; adding it is the same either way.
  const addField = useCallback(
    (field: string) => {
      let added:
        | { expr: FilterExpression; itemKey: string; filterType: string }
        | undefined;
      if (searchActive) {
        if (searchFilterNext.includes(field)) {
          added = {
            expr: addSearchFilterClause(
              shown,
              field,
              defaultSearchFilterValue(field, defaultDocument),
            ),
            itemKey: `searchFilter/${enabledSearchClauses(shown).length}`,
            filterType: "searchFilter",
          };
        }
      } else if (indexedNext.some((n) => n.field === field)) {
        added = {
          expr: addIndexedClause(
            indexDefs,
            shown,
            field,
            defaultIndexedClause(field, defaultDocument),
          ),
          itemKey: `indexed/${used.length}`,
          filterType: "index",
        };
      }
      if (!added) return false;
      commitLive(added.expr);
      setOpenItemKey(added.itemKey);
      log("filter add", { filterType: added.filterType });
      return true;
    },
    [
      commitLive,
      defaultDocument,
      indexDefs,
      indexedNext,
      log,
      searchActive,
      searchFilterNext,
      shown,
      used.length,
    ],
  );

  // Adds an unindexed clause on `field` on top of whatever the index
  // returns, and opens its editor.
  const addScanField = useCallback(
    (field: string) => {
      const clause = defaultScanClause(field, defaultDocument);
      commitLive(addScanClause(shown, clause));
      setOpenItemKey(scanKey(clause, shown.clauses.length));
      log("filter add", { filterType: "regular" });
    },
    [commitLive, defaultDocument, log, shown],
  );

  // Switches to `index` (keeping the applied clauses it shares) and adds an
  // indexed clause on `field`.
  const addFieldWithIndex = useCallback(
    (index: DatabaseIndexDef, field: string) => {
      const next = switchIndexAndAddClause(
        indexDefs,
        shown,
        index,
        field,
        defaultIndexedClause(field, defaultDocument),
      );
      commitLive(next);
      setOpenItemKey(`indexed/${enabledIndexClauses(next).length - 1}`);
      log("filter add", { filterType: "index", switchedIndex: index.name });
    },
    [commitLive, defaultDocument, indexDefs, log, shown],
  );

  const indexesFor = useCallback(
    (field: string): IndexForField[] =>
      indexesForField(indexDefs, shown, field),
    [indexDefs, shown],
  );

  // Adds a complete clause from a cell's context menu and applies it: through
  // the index when the field is its next one, otherwise as a scan on top.
  const addComplete = useCallback(
    (filter: Filter): boolean => {
      const { field, op } = filter;
      if (field === undefined) return false;
      if (searchActive) {
        if (op !== "eq" || !searchFilterNext.includes(field)) return false;
        commitLive(
          addSearchFilterClause(shown, field, filter.value as JSONValue),
        );
        log("filter add", { filterType: "searchFilter", source: "table" });
        return true;
      }
      const value = filter.value as JSONValue | undefined;
      if (indexedNext.some((n) => n.field === field)) {
        let clause: DatabaseIndexFilterClause | undefined;
        switch (op) {
          case "eq":
            clause = { type: "indexEq", enabled: true, value };
            break;
          case "gt":
          case "gte":
            clause = {
              type: "indexRange",
              enabled: true,
              lowerOp: op,
              lowerValue: value,
            };
            break;
          case "lt":
          case "lte":
            clause = {
              type: "indexRange",
              enabled: true,
              upperOp: op,
              upperValue: value,
            };
            break;
          default:
            clause = undefined;
        }
        if (clause) {
          commitLive(addIndexedClause(indexDefs, shown, field, clause));
          log("filter add", { filterType: "index", source: "table" });
          return true;
        }
      }
      commitLive(
        addScanClause(shown, {
          ...filter,
          id: filter.id ?? newClauseId(),
          enabled: true,
        } as Filter),
      );
      log("filter add", { filterType: "regular", source: "table" });
      return true;
    },
    [
      commitLive,
      indexDefs,
      indexedNext,
      log,
      searchActive,
      searchFilterNext,
      shown,
    ],
  );

  const updateItem = useCallback(
    (key: string, update: FilterItemUpdate) => {
      const item = items.find((c) => c.key === key);
      if (!item) return;
      let next: FilterExpression | undefined;
      if (item.kind === "indexed" && update.kind === "indexed") {
        next = setIndexedClause(indexDefs, shown, item.position, update.clause);
      } else if (item.kind === "scan" && update.kind === "scan") {
        next = setScanClause(shown, item.position, update.clause);
      } else if (item.kind === "search" && update.kind === "search") {
        next = setSearchText(shown, update.search);
      } else if (
        item.kind === "searchFilter" &&
        update.kind === "searchFilter"
      ) {
        next = setSearchFilterClause(shown, item.position, update.value);
      }
      // Editors report their initial value on mount; that isn't a change and
      // must not restart the debounce.
      if (next && !isEqual(next, shown)) {
        setDraftFilters(next);
        scheduleApply(next);
      }
    },
    [items, indexDefs, scheduleApply, setDraftFilters, shown],
  );

  // Indexed filters (and the search a search index's filters hang off) are
  // removed from the end only, so no removal ever takes other clauses with it.
  const removeItem = useCallback(
    (key: string) => {
      const item = items.find((c) => c.key === key);
      if (!item) return;
      switch (item.kind) {
        case "indexed": {
          if (items.findLast((c) => c.kind === "indexed") !== item) return;
          const { expr } = removeIndexedClause(indexDefs, shown, item.position);
          commitLive(expr);
          log("filter delete", { filterType: "index" });
          break;
        }
        case "scan":
          commitLive(removeScanClause(shown, item.position));
          log("filter delete", { filterType: "regular" });
          break;
        case "search":
          if (
            isSearchFilter(shown.index) &&
            shown.index.clauses.some((c) => c.enabled)
          ) {
            return;
          }
          commitLive(clearSearchIndex(shown));
          log("filter delete", { filterType: "search" });
          break;
        case "searchFilter":
          commitLive(removeSearchFilterClause(shown, item.position));
          log("filter delete", { filterType: "searchFilter" });
          break;
        default:
          return;
      }
      setErrors((prev) => ({ ...prev, [key]: undefined }));
      if (openItemKey === key) setOpenItemKey(null);
    },
    [items, commitLive, indexDefs, log, openItemKey, shown],
  );

  const startSearch = useCallback(
    (def: SearchIndexDef) => {
      commitLive(setSearchIndex(shown, def));
      setOpenItemKey("search");
      log("sort by index combobox opened", { selectedOption: def.name });
    },
    [commitLive, log, shown],
  );

  const chooseIndex = useCallback(
    (def: DatabaseIndexDef) => {
      commitLive(setDatabaseIndex(indexDefs, shown, def));
      log("sort by index combobox opened", { selectedOption: def.name });
    },
    [commitLive, indexDefs, log, shown],
  );

  const setOrder = useCallback(
    (order: "asc" | "desc") => {
      log("filter order change", { oldOrder: shown.order, newOrder: order });
      commitLive(setOrderInModel(shown, order));
    },
    [commitLive, log, shown],
  );

  const sortByField = useCallback(
    (field: string) => {
      const next = sortByFieldInModel(indexDefs, shown, field);
      if (!next) return false;
      log("filter order change", { source: "header", field });
      commitLive(next);
      return true;
    },
    [commitLive, indexDefs, log, shown],
  );

  const sortOptionFor = useCallback(
    (field: string): SortOption => sortOptionForField(indexDefs, shown, field),
    [indexDefs, shown],
  );

  // Closing an item's editor flushes any value still waiting on the debounce.
  const closeItem = useCallback(() => {
    setOpenItemKey(null);
    flushPendingApply();
  }, [flushPendingApply]);

  return {
    shown,
    applied,
    items,
    errors,
    hasInvalid,
    openItemKey,
    setOpenItemKey,
    closeItem,
    setItemError,
    indexedNext,
    searchFilterNext,
    nextFields,
    searchActive,
    lastIsRange,
    currentIndex,
    addField,
    addScanField,
    addFieldWithIndex,
    indexesFor,
    addComplete,
    updateItem,
    removeItem,
    startSearch,
    chooseIndex,
    setOrder,
    sortByField,
    sortOptionFor,
  };
}

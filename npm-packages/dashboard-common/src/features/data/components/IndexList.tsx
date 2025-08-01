import groupBy from "lodash/groupBy";
import {
  MagnifyingGlassIcon,
  QuestionMarkCircledIcon,
  ArrowTopRightIcon,
} from "@radix-ui/react-icons";
import { FingerPrintIcon } from "@heroicons/react/24/outline";
import { Index } from "@common/features/data/lib/api";
import { useNents } from "@common/lib/useNents";
import { Loading } from "@ui/Loading";
import { Spinner } from "@ui/Spinner";
import { useQuery } from "convex/react";
import { api } from "system-udfs/convex/_generated/api";
import { Fragment } from "react";
import { ProgressBar } from "@ui/ProgressBar";
import { Tooltip } from "@ui/Tooltip";

export function IndexList({ tableName }: { tableName: string }) {
  const { selectedNent } = useNents();
  const indexes =
    useQuery(api._system.frontend.indexes.default, {
      tableName,
      tableNamespace: selectedNent?.id ?? null,
    }) ?? undefined;

  return <IndexesList tableName={tableName} indexes={indexes} />;
}

export function IndexesList({
  tableName,
  indexes,
}: {
  tableName: string;
  indexes: Index[] | undefined;
}) {
  if (indexes === undefined) {
    return <Loading />;
  }

  const groupedIndexes = groupBy(indexes, getIndexType);

  return (
    <div className="flex flex-col gap-10">
      <IndexListSection
        title="Indexes"
        description="Indexes allow you to speed up your document queries by telling Convex how to organize your documents."
        learnMoreUrl="https://docs.convex.dev/database/reading-data/indexes/"
        indexes={groupedIndexes.database ?? []}
        icon={FingerPrintIcon}
        tableName={tableName}
      />
      <IndexListSection
        title="Search indexes"
        description="Search indexes allows you to find Convex documents that approximately match a textual search query."
        learnMoreUrl="https://docs.convex.dev/search/text-search"
        indexes={groupedIndexes.search ?? []}
        icon={MagnifyingGlassIcon}
        tableName={tableName}
      />
      <IndexListSection
        title="Vector indexes"
        description="Vector search allows you to find Convex documents similar to a provided vector."
        learnMoreUrl="https://docs.convex.dev/search/vector-search"
        indexes={groupedIndexes.vector ?? []}
        icon={ArrowTopRightIcon}
        tableName={tableName}
      />
    </div>
  );
}

function IndexListSection({
  title,
  description,
  learnMoreUrl,
  indexes,
  icon: Icon,
  tableName,
}: {
  title: string;
  description: string;
  learnMoreUrl: string;
  indexes: Index[];
  icon: React.FC<{ className?: string }>;
  tableName: string;
}) {
  const indexesByName = groupBy(indexes, "name");

  return (
    <div className="flex flex-col gap-3">
      <header className="flex items-center gap-1.5 text-content-primary">
        <Icon className="size-5 text-content-secondary" />
        <h5 className="text-base font-medium">{title}</h5>
        <Tooltip
          tip={
            <p>
              {description}{" "}
              <a
                href={learnMoreUrl}
                target="_blank"
                rel="noopener noreferrer"
                className="text-content-link hover:underline"
              >
                Learn more
              </a>
            </p>
          }
        >
          <QuestionMarkCircledIcon className="text-content-tertiary" />
        </Tooltip>
      </header>
      {indexes.length === 0 ? (
        <div className="text-sm text-content-tertiary">
          <code>{tableName}</code> has no {title.toLowerCase()}.
        </div>
      ) : (
        <div className="flex flex-col gap-5">
          {indexes.map((index) => (
            <IndexListRow
              key={`${index.name} ${indexesByName[index.name].indexOf(index)}`}
              index={index}
            />
          ))}
        </div>
      )}
    </div>
  );
}

function IndexListRow({ index }: { index: Index }) {
  const { fields } = index;

  return (
    <article className="flex flex-col gap-2 text-sm text-content-secondary">
      <header className="flex items-center gap-2">
        <h6 className="truncate font-mono text-sm font-medium text-content-primary">
          {index.name}
        </h6>
        <div className="grow border-b" role="presentation" />
      </header>

      <div className="flex flex-col gap-1 pl-2">
        {Array.isArray(fields) && (
          <IndexAttribute title="Fields">
            <FieldList fields={fields} />
          </IndexAttribute>
        )}

        {"searchField" in fields && (
          <IndexAttribute title="Search field">
            <code>{fields.searchField}</code>
          </IndexAttribute>
        )}

        {"vectorField" in fields && (
          <IndexAttribute title="Vector field">
            <code>{fields.vectorField}</code>
          </IndexAttribute>
        )}

        {"filterFields" in fields && fields.filterFields.length > 0 && (
          <IndexAttribute title="Filter fields">
            <FieldList fields={fields.filterFields} />
          </IndexAttribute>
        )}

        {"dimensions" in fields && (
          <IndexAttribute title="Dimensions">
            {fields.dimensions}
          </IndexAttribute>
        )}
      </div>

      {index.backfill.state === "in_progress" && (
        <div className="flex flex-col gap-1 pl-2">
          <p className="flex items-center gap-2">
            {!(
              index.backfill.stats && index.backfill.stats.totalDocs !== null
            ) && (
              <div>
                <Spinner />
              </div>
            )}
            Backfill in progress
          </p>
          {index.backfill.stats && index.backfill.stats.totalDocs !== null && (
            <IndexBackfillProgress
              numDocsIndexed={index.backfill.stats.numDocsIndexed}
              totalDocs={index.backfill.stats.totalDocs}
            />
          )}
        </div>
      )}
    </article>
  );
}

function IndexAttribute({
  title,
  children,
}: React.PropsWithChildren<{ title: string }>) {
  return (
    <div className="flex gap-1">
      <span>
        <strong className="font-medium text-content-primary">{title}</strong>:
      </span>
      <div>{children}</div>
    </div>
  );
}

function FieldList({ fields }: { fields: string[] }) {
  return (
    <>
      {fields.map((field) => (
        <Fragment key={field}>
          <code>{field}</code>
          {fields.length > 1 && fields.indexOf(field) < fields.length - 1 && (
            <span>, </span>
          )}
        </Fragment>
      ))}
    </>
  );
}

function IndexBackfillProgress({
  numDocsIndexed,
  totalDocs,
}: {
  numDocsIndexed: number;
  totalDocs: number;
}) {
  const fraction = Math.min(numDocsIndexed / totalDocs, 0.99);
  const percent = Math.round(fraction * 100);
  return (
    <div className="flex items-center gap-4">
      <ProgressBar
        fraction={fraction}
        ariaLabel="Index backfill progress"
        variant="stripes"
        className="grow"
      />
      <span className="text-xs text-content-tertiary">{percent}%</span>
    </div>
  );
}

function getIndexType(index: Index) {
  if (Array.isArray(index.fields)) {
    return "database" as const;
  }

  if ("searchField" in index.fields) {
    return "search" as const;
  }

  if ("vectorField" in index.fields) {
    return "vector" as const;
  }

  const _unreachable: never = index.fields;
  return "unknown" as const;
}

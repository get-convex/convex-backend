import { Period, UsagePeriodSelector } from "elements/UsagePeriodSelector";
import { Combobox } from "@ui/Combobox";
import { Tooltip } from "@ui/Tooltip";
import { TextInput } from "@ui/TextInput";
import { useRouter } from "next/router";
import { ProjectDetails } from "generatedApi";
import { PuzzlePieceIcon } from "@common/elements/icons";

export function TeamUsageToolbar({
  shownBillingPeriod,
  setSelectedBillingPeriod,
  currentBillingPeriod,
  projects,
  projectId,
}: {
  shownBillingPeriod: Period;
  projects: ProjectDetails[];
  projectId: number | null;
  setSelectedBillingPeriod: (period: Period) => void;
  currentBillingPeriod: { start: string; end: string };
}) {
  const { query, replace } = useRouter();

  // Detect duplicate project names
  const nameCountMap = new Map<string, number>();
  projects.forEach((p) => {
    nameCountMap.set(p.name, (nameCountMap.get(p.name) || 0) + 1);
  });

  return (
    <div className="sticky top-0 z-20 mb-6 flex h-(--team-usage-toolbar-height) flex-wrap content-center items-center gap-2 border-b bg-background-primary">
      <UsagePeriodSelector
        period={shownBillingPeriod}
        onChange={setSelectedBillingPeriod}
        currentBillingPeriod={currentBillingPeriod}
      />
      <Combobox
        label="Projects"
        options={[
          { label: "All Projects", value: null },
          ...projects.map((p) => {
            const isDuplicate = (nameCountMap.get(p.name) || 0) > 1;
            const label =
              isDuplicate && p.slug ? `${p.name} (${p.slug})` : p.name;
            return { label, value: p.id };
          }),
        ]}
        allowCustomValue
        selectedOption={projectId}
        innerButtonClasses={
          projectId
            ? "bg-yellow-100/50 dark:bg-yellow-600/20 hover:bg-yellow-100 dark:hover:bg-yellow-600/50"
            : ""
        }
        setSelectedOption={(o) => {
          const newProject = projects?.find((p) => p.id === o);
          query.projectSlug = newProject?.slug ?? o?.toString();
          void replace({ query }, undefined, { shallow: true });
        }}
        unknownLabel={() => "projects"}
      />

      <Tooltip
        className="w-[12rem] animate-fadeInFromLoading"
        tip={
          <div className="flex flex-col gap-1">
            <p>
              Filter usage to only include components whose paths start with
              this string.
            </p>
            <p>Enter "app" to only see results for the root app.</p>
          </div>
        }
      >
        <TextInput
          label="Component Prefix"
          labelHidden
          id="componentPrefix"
          type="search"
          SearchIcon={PuzzlePieceIcon}
          placeholder="Component Prefix"
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.currentTarget.blur();
            }
          }}
          onBlur={(e) => {
            const { value } = e.target;
            query.componentPrefix = value;
            void replace({ query }, undefined, { shallow: true });
          }}
        />
      </Tooltip>

      <span className="text-sm text-content-secondary lg:ml-auto">
        All dates are in UTC
      </span>
    </div>
  );
}

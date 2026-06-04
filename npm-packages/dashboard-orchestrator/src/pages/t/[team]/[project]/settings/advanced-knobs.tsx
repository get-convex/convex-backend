import Head from "next/head";
import { useRouter } from "next/router";
import { useEffect, useMemo, useState } from "react";
import { Sheet } from "@ui/Sheet";
import { Button } from "@ui/Button";
import { Callout } from "@ui/Callout";
import { TextInput } from "@ui/TextInput";
import { Link as UiLink } from "@ui/Link";
import useSWR from "swr";
import { listProjects, listTeams } from "../../../../../lib/orchestratorApi";
import { useAccessToken } from "../../../../../lib/useOrchestratorToken";
import { orchestratorUrl } from "../../../../../lib/config";
import { useHostCapacity } from "../../../../../hooks/useHostCapacity";
import { useKnobRegistry } from "../../../../../hooks/useKnobRegistry";
import { useProjectSettings } from "../../../../../hooks/useProjectSettings";
import { KnobRow } from "../../../../../components/backendSettings/KnobRow";
import { clearVisibleOverrides } from "../../../../../components/backendSettings/knobOverrides";
import { tierDefaultsForName } from "../../../../../components/backendSettings/tiers";
import {
  advancedKnobRowState,
  filterAdvancedKnobs,
  visibleOverrideCount,
  type AdvancedKnobShowFilter,
} from "../../../../../components/backendSettings/advancedKnobs";

export default function AdvancedKnobsPage() {
  const router = useRouter();
  const teamSlug = router.query.team as string | undefined;
  const projectSlug = router.query.project as string | undefined;
  const token = useAccessToken();
  const url = orchestratorUrl();

  const { data: teams } = useSWR(token ? ["teams", token] : null, () =>
    listTeams(url, token!),
  );
  const team = useMemo(
    () => teams?.find((t) => t.slug === teamSlug),
    [teams, teamSlug],
  );
  const { data: projects } = useSWR(
    team && token ? ["projects", team.id, token] : null,
    () => listProjects(url, token!, team!.id),
  );
  const project = useMemo(
    () => projects?.find((p) => p.slug === projectSlug),
    [projects, projectSlug],
  );

  const { settings, save } = useProjectSettings(project?.id);
  const { data: registry } = useKnobRegistry();
  const { data: capacity } = useHostCapacity();

  const [search, setSearch] = useState("");
  const [category, setCategory] = useState<string>("ALL");
  const [show, setShow] = useState<AdvancedKnobShowFilter>("all");
  const [draft, setDraft] = useState<Record<string, string> | null>(null);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (settings && draft === null) {
      setDraft({ ...settings.knobOverrides });
    }
  }, [settings, draft]);

  const categories = useMemo(() => {
    const set = new Set<string>(registry?.map((k) => k.category) ?? []);
    return ["ALL", ...Array.from(set).sort()];
  }, [registry]);

  const settingsTier = settings?.tier;
  const tierDefaults = useMemo(
    () => (settingsTier ? tierDefaultsForName(settingsTier) : {}),
    [settingsTier],
  );

  const filtered = useMemo(() => {
    if (!registry) return [];
    return filterAdvancedKnobs({
      registry,
      overrides: draft ?? {},
      search,
      category,
      show,
    });
  }, [registry, search, category, show, draft]);

  const overriddenCount =
    draft && registry ? visibleOverrideCount(draft, registry) : 0;
  const dirty =
    !!settings &&
    !!draft &&
    JSON.stringify(draft) !== JSON.stringify(settings.knobOverrides);

  const onSave = async () => {
    if (!draft || !settings) return;
    setError(null);
    setSaving(true);
    try {
      const patch: Record<string, string | null> = { ...draft };
      for (const k of Object.keys(settings.knobOverrides)) {
        if (!(k in draft)) patch[k] = null;
      }
      await save({ knobOverrides: patch });
    } catch (err) {
      setError((err as Error).message);
    } finally {
      setSaving(false);
    }
  };

  if (!team || !project) return null;

  return (
    <>
      <Head>
        <title>Advanced backend settings | {project.name}</title>
      </Head>
      <div className="m-auto flex max-w-(--container-width) flex-col gap-4 p-6 [--container-width:80rem]">
        <UiLink href={`/t/${team.slug}/${project.slug}/settings#backend`}>
          ← Project settings
        </UiLink>
        {/* eslint-disable-next-line no-restricted-syntax -- text-lg IS the heading style on this h1 */}
        <h1 className="text-lg font-semibold">Advanced backend settings</h1>
        <Callout variant="instructions">
          These are power-user knobs. Bad values can prevent a deployment from
          starting. Changes apply to new deployments only.
        </Callout>
        <Sheet>
          <div className="mb-3 flex flex-wrap items-center gap-2">
            <div className="grow">
              <TextInput
                id="knobSearch"
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                placeholder="Search knobs…"
              />
            </div>
            {}
            <select
              value={category}
              onChange={(e) => setCategory(e.target.value)}
              className="h-9 rounded-sm border border-border-transparent bg-background-primary px-2 text-sm"
            >
              {categories.map((c) => (
                <option key={c} value={c}>
                  {c}
                </option>
              ))}
            </select>
            {}
            <select
              value={show}
              onChange={(e) => setShow(e.target.value as typeof show)}
              className="h-9 rounded-sm border border-border-transparent bg-background-primary px-2 text-sm"
            >
              <option value="all">All</option>
              <option value="overridden">Overridden only</option>
              <option value="curated">Curated</option>
              <option value="tier">Tier-tuned</option>
            </select>
            {overriddenCount > 0 && (
              <Button
                variant="neutral"
                size="xs"
                onClick={() =>
                  setDraft((d) =>
                    registry ? clearVisibleOverrides(d ?? {}, registry) : d,
                  )
                }
              >
                Revert all to defaults
              </Button>
            )}
          </div>
          <div className="mb-2 text-xs text-content-secondary">
            Showing {filtered.length} of {registry?.length ?? 0} ·{" "}
            {overriddenCount} overridden
          </div>
          <div className="divide-y">
            {filtered.map((knob) => {
              const rowState = advancedKnobRowState(
                knob,
                draft ?? {},
                tierDefaults,
              );
              return (
                <KnobRow
                  key={knob.envVar}
                  knob={knob}
                  source={rowState.source}
                  effectiveValue={rowState.effectiveValue}
                  overrideValue={rowState.overrideValue}
                  onOverride={(next) =>
                    setDraft((d) => ({ ...(d ?? {}), [knob.envVar]: next }))
                  }
                  onReset={() =>
                    setDraft((d) => {
                      if (!d) return d;
                      const { [knob.envVar]: _, ...rest } = d;
                      return rest;
                    })
                  }
                />
              );
            })}
          </div>
        </Sheet>
        {error && (
          <div className="text-xs text-content-error" role="alert">
            {error}
          </div>
        )}
        <div className="bottom-four sticky flex justify-end gap-2">
          <Button
            variant="neutral"
            size="xs"
            disabled={!dirty || saving}
            onClick={() => setDraft({ ...(settings?.knobOverrides ?? {}) })}
          >
            Discard
          </Button>
          <Button size="xs" disabled={!dirty || saving} onClick={onSave}>
            {saving ? "Saving…" : "Save"}
          </Button>
        </div>
        {capacity && (
          <div className="text-xs text-content-secondary">
            Host: {(capacity.allocatedMemoryMb / 1024).toFixed(1)} /{" "}
            {(capacity.totalMemoryMb / 1024).toFixed(1)} GB allocated ·{" "}
            {capacity.deploymentCount} deployments
          </div>
        )}
      </div>
    </>
  );
}

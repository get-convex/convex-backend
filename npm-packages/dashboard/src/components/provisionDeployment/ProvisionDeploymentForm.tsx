import {
  useEffect,
  useMemo,
  useState,
  useRef,
  useId,
  useCallback,
} from "react";
import { Button } from "@ui/Button";
import { Checkbox } from "@ui/Checkbox";
import { Tooltip } from "@ui/Tooltip";
import type { DeploymentRegionMetadata } from "@convex-dev/platform/managementApi";
import type { RegionName } from "generatedApi";
import { useCurrentTeam, useUpdateTeam } from "api/teams";
import { useIsCurrentMemberTeamAdmin } from "api/roles";
import { useRouter } from "next/router";
import { useProvisionDeployment } from "api/deployments";
import { Fieldset, Legend, RadioGroup } from "@headlessui/react";
import { cn } from "@ui/cn";
import { Sheet } from "@ui/Sheet";
import { useCurrentTheme } from "@common/lib/useCurrentTheme";
import createGlobe from "cobe";
import { SignalIcon } from "@heroicons/react/24/outline";
import { GlobeIcon } from "@radix-ui/react-icons";
import { Region } from "elements/Region";
import { RegionPricingWarning } from "elements/RegionPricingWarning";
import {
  DEFAULT_GLOBE_COORDINATES,
  getRegionCoordinates,
  useEnabledRegionCoordinates,
  useSelectableRegions,
} from "lib/regions";

export function ProvisionDeploymentForm({
  projectId,
  projectURI,
  deploymentType,
}: {
  projectId: number;
  projectURI: string;
  deploymentType: "prod" | "dev";
}) {
  const router = useRouter();
  const team = useCurrentTeam();
  const provisionDeployment = useProvisionDeployment(projectId);
  const updateTeam = useUpdateTeam(team?.id ?? 0, /* toast */ false);
  const isAdmin = useIsCurrentMemberTeamAdmin();

  const { regions, expectedRegionCount } = useSelectableRegions(team?.id);

  const handleCreate = useCallback(
    async (region: string, setAsDefault: boolean) => {
      if (setAsDefault) {
        await updateTeam({ defaultRegion: region as RegionName });
      }
      const { name } = await provisionDeployment({
        type: deploymentType,
        region: region as RegionName,
      });
      void router.replace(`${projectURI}/${name}`);
    },
    [updateTeam, provisionDeployment, deploymentType, router, projectURI],
  );

  return (
    <ProvisionDeploymentFormInner
      deploymentType={deploymentType}
      regions={regions}
      expectedRegionCount={expectedRegionCount}
      onCreate={handleCreate}
      teamSlug={team?.slug}
      teamName={team?.name}
      isAdmin={isAdmin}
    />
  );
}

export function ProvisionDeploymentFormInner({
  deploymentType,
  regions,
  expectedRegionCount,
  onCreate,
  teamSlug,
  teamName,
  isAdmin,
}: {
  deploymentType: "prod" | "dev";
  regions: DeploymentRegionMetadata[] | undefined;
  /** How many region tiles to render while `regions` loads. */
  expectedRegionCount: number;
  onCreate: (region: string, setAsDefault: boolean) => Promise<void>;
  teamSlug: string | undefined;
  teamName: string | undefined;
  isAdmin: boolean;
}) {
  const [selectedRegion, setSelectedRegion] = useState<RegionName | null>(null);
  const [isCreating, setIsCreating] = useState(false);
  const [setAsDefault, setSetAsDefault] = useState(false);

  // When the user is an admin, default “set as default” to true
  useEffect(() => {
    setSetAsDefault(isAdmin);
  }, [isAdmin]);

  // Select the first region by default (will be us-east in prod)
  useEffect(() => {
    if (!selectedRegion && regions && regions.length > 0) {
      setSelectedRegion(regions[0].name);
    }
  }, [regions, selectedRegion]);

  const defaultCheckboxId = useId();

  return (
    <div className="flex size-full justify-center">
      <div className="my-auto flex w-full max-w-xl flex-col gap-6 p-4">
        <Sheet className="relative overflow-hidden">
          <Globe selectedRegion={selectedRegion} />
          <form
            className="relative flex flex-col gap-6 p-3"
            onSubmit={async (e: React.FormEvent<HTMLFormElement>) => {
              e.preventDefault();
              if (!selectedRegion) {
                return;
              }
              setIsCreating(true);
              try {
                await onCreate(selectedRegion, setAsDefault);
              } catch (error) {
                setIsCreating(false);
                throw error;
              }
            }}
          >
            <h3 className="flex flex-col gap-0.5">
              <span>Create a new deployment</span>
              <span
                className={cn(
                  "inline-flex items-center gap-1.5",
                  deploymentType === "prod"
                    ? "text-purple-600 dark:text-purple-100"
                    : "text-green-600 dark:text-green-400",
                )}
              >
                {deploymentType === "prod" ? (
                  <SignalIcon className="size-4 shrink-0" />
                ) : (
                  <GlobeIcon className="size-4 shrink-0" />
                )}
                {deploymentType === "prod" ? "Production" : "Development"}
              </span>
            </h3>
            <Fieldset>
              <Legend className="mb-1 text-sm text-content-primary">
                Region
              </Legend>
              <RadioGroup
                name="region"
                value={selectedRegion ?? null}
                onChange={setSelectedRegion}
              >
                <div className="grid auto-rows-fr grid-cols-1 gap-4 sm:grid-cols-2">
                  {regions === undefined
                    ? Array.from({ length: expectedRegionCount }, (_, i) => (
                        <Region
                          key={i}
                          region={undefined}
                          teamSlug={teamSlug}
                        />
                      ))
                    : regions.map((region) => (
                        <Region
                          key={region.name}
                          region={region}
                          teamSlug={teamSlug}
                        />
                      ))}
                </div>
              </RadioGroup>
              <RegionPricingWarning region={selectedRegion} />
            </Fieldset>

            <Tooltip
              tip={
                isAdmin
                  ? undefined
                  : "You do not have permission to update the region for new deployments."
              }
            >
              <label
                htmlFor={defaultCheckboxId}
                className={cn(
                  "flex items-start gap-2 text-sm",
                  !isAdmin && "cursor-not-allowed opacity-50",
                )}
              >
                {/* align with the first line of the paragraph */}
                <div className="mt-[0.2rem] flex">
                  <Checkbox
                    id={defaultCheckboxId}
                    checked={setAsDefault}
                    onChange={() => setSetAsDefault(!setAsDefault)}
                    disabled={!isAdmin}
                  />
                </div>
                <p className="mt-0 text-left">
                  Use this region for all new deployments in{" "}
                  <span className="font-medium">{teamName}</span>
                </p>
              </label>
            </Tooltip>

            <div>
              <Button
                type="submit"
                disabled={!selectedRegion}
                loading={isCreating}
              >
                Create deployment
              </Button>
            </div>
          </form>
        </Sheet>
      </div>
    </div>
  );
}

function Globe({ selectedRegion }: { selectedRegion: RegionName | null }) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const focusRef = useRef<[number, number]>([0, 0]);
  const currentTheme = useCurrentTheme();
  const isDark = currentTheme === "dark";

  // Derived from the launch flags rather than the fetched region list, so the
  // globe isn't torn down and rebuilt when that request lands.
  const regionCoordinates = useEnabledRegionCoordinates();
  const markers = useMemo(
    () => regionCoordinates.map((location) => ({ location, size: 0.07 })),
    [regionCoordinates],
  );

  // Update focus when region changes
  useEffect(() => {
    const coordinates = selectedRegion && getRegionCoordinates(selectedRegion);
    if (coordinates) {
      focusRef.current = locationToAngles(...coordinates);
    }
  }, [selectedRegion]);

  useEffect(() => {
    if (!canvasRef.current) return;

    let windowWidth = 0;
    focusRef.current = locationToAngles(...DEFAULT_GLOBE_COORDINATES);
    let [currentPhi, currentTheta] = [...focusRef.current];
    const doublePi = Math.PI * 2;

    const onResize = () => {
      if (canvasRef.current) {
        windowWidth = window.innerWidth;
      }
    };
    window.addEventListener("resize", onResize);
    onResize();

    const globe = createGlobe(canvasRef.current, {
      devicePixelRatio: 2,
      width: 0,
      height: 0,
      scale: 0,
      phi: currentPhi,
      theta: currentTheta,
      dark: 0,
      diffuse: isDark ? 3 : 7,
      mapSamples: 20000,
      mapBrightness: isDark ? 6 : 4,
      baseColor: isDark ? [0.3, 0.3, 0.3] : [1, 1, 1],
      markerColor: [0.5, 0.5, 0.5],
      glowColor: isDark
        ? [42 / 255, 40 / 255, 37 / 255]
        : [253 / 255, 252 / 255, 250 / 255],
      markers,
      onRender: (state) => {
        state.phi = currentPhi;
        state.theta = currentTheta;
        const [focusPhi, focusTheta] = focusRef.current;
        const distPositive = (focusPhi - currentPhi + doublePi) % doublePi;
        const distNegative = (currentPhi - focusPhi + doublePi) % doublePi;

        const speed = 0.03;

        if (distPositive < distNegative) {
          currentPhi += distPositive * speed;
        } else {
          currentPhi -= distNegative * speed;
        }
        currentTheta = currentTheta * (1 - speed) + focusTheta * speed;

        const sm = windowWidth >= 640; // from Tailwind
        state.width = sm ? 900 : 900;
        state.height = sm ? 900 : 900;
        state.offset = sm ? [900, -320] : [500, -410];
        state.scale = sm ? 1.15 : 1.1;

        state.mapSamples = sm ? 25000 : 20000;
      },
    });

    setTimeout(() => {
      if (canvasRef.current) {
        canvasRef.current.style.opacity = "1";
      }
    });

    return () => {
      globe.destroy();
      window.removeEventListener("resize", onResize);
    };
  }, [isDark, markers]);

  return (
    <canvas
      className="pointer-events-none absolute inset-0 size-full"
      aria-hidden
      ref={canvasRef}
      style={{
        opacity: 0,
        transition: "opacity 1s ease",
      }}
    />
  );
}

function locationToAngles(lat: number, long: number): [number, number] {
  return [
    Math.PI - ((long * Math.PI) / 180 - Math.PI / 2),
    (lat * Math.PI) / 180,
  ];
}

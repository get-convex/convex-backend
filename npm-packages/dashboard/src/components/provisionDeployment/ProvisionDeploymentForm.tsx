import { useEffect, useMemo, useState, useRef } from "react";
import { Button } from "@ui/Button";
import { useManagementApiQuery } from "api/api";
import type { DeploymentRegionMetadata } from "@convex-dev/platform/managementApi";
import type { RegionName } from "generatedApi";
import { useCurrentTeam } from "api/teams";
import { useRouter } from "next/router";
import { useProvisionDeployment } from "api/deployments";
import { Fieldset, Legend, RadioGroup } from "@headlessui/react";
import { cn } from "@ui/cn";
import { Sheet } from "@ui/Sheet";
import { Loading } from "@ui/Loading";
import { useTheme } from "next-themes";
import createGlobe from "cobe";
import { SignalIcon } from "@heroicons/react/24/outline";
import { GlobeIcon } from "@radix-ui/react-icons";
import { Region } from "./Region";

const REGION_ORDER = ["aws-us-east-1", "aws-eu-west-1"];

const REGION_COORDINATES: Record<RegionName, [number, number]> = {
  "aws-us-east-1": [38.9072, -77.0369], // Washington DC area (US East)
  "aws-eu-west-1": [53.3498, -6.2603], // Dublin (EU West)
};

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

  const { data: regionsData } = useManagementApiQuery({
    path: "/teams/{team_id}/list_deployment_regions",
    pathParams: { team_id: team?.id?.toString() ?? "paused" },
    swrOptions: {
      isPaused: () => !team?.id,
    },
  });

  const handleCreate = async (region: string) => {
    const { name } = await provisionDeployment({
      type: deploymentType,
      region: region as RegionName,
    });
    void router.replace(`${projectURI}/${name}`);
  };

  return (
    <ProvisionDeploymentFormInner
      deploymentType={deploymentType}
      regions={regionsData?.items}
      onCreate={handleCreate}
      teamSlug={team?.slug}
    />
  );
}

export function ProvisionDeploymentFormInner({
  deploymentType,
  regions,
  onCreate,
  teamSlug,
}: {
  deploymentType: "prod" | "dev";
  regions: DeploymentRegionMetadata[] | undefined;
  onCreate: (region: string) => Promise<void>;
  teamSlug?: string;
}) {
  const sortedRegions = useMemo(() => {
    if (!regions) return undefined;
    return [...regions].sort((a, b) => {
      const aIndex = REGION_ORDER.indexOf(a.name);
      const bIndex = REGION_ORDER.indexOf(b.name);
      if (aIndex !== -1 && bIndex !== -1) return aIndex - bIndex;
      if (aIndex !== -1) return -1;
      if (bIndex !== -1) return 1;
      return 0;
    });
  }, [regions]);

  const [selectedRegion, setSelectedRegion] = useState<string | null>(null);
  const [isCreating, setIsCreating] = useState(false);

  useEffect(() => {
    if (!selectedRegion && sortedRegions && sortedRegions.length > 0) {
      setSelectedRegion(sortedRegions[0].name);
    }
  }, [sortedRegions, selectedRegion]);

  return (
    <div className="flex size-full justify-center">
      <div className="my-auto flex w-full max-w-xl flex-col gap-6 p-4">
        <Sheet className="relative">
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
                await onCreate(selectedRegion);
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
              <Legend className="mb-3 text-sm font-medium">Region</Legend>
              {sortedRegions === undefined ? (
                <div className="grid grid-cols-1 items-stretch gap-4 sm:grid-cols-2">
                  {[1, 2].map((i) => (
                    <Loading
                      key={i}
                      className="h-full min-h-[60px] rounded-xl border bg-background-secondary"
                      fullHeight={false}
                    />
                  ))}
                </div>
              ) : (
                <RadioGroup
                  name="region"
                  value={selectedRegion ?? ""}
                  onChange={setSelectedRegion}
                >
                  <div className="grid grid-cols-1 items-stretch gap-4 sm:grid-cols-2">
                    {sortedRegions.map((region) => (
                      <Region
                        key={region.name}
                        region={region}
                        teamSlug={teamSlug}
                      />
                    ))}
                  </div>
                </RadioGroup>
              )}
            </Fieldset>

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
  const { forcedTheme, resolvedTheme } = useTheme();
  const isDark = (forcedTheme ?? resolvedTheme) === "dark";

  // Update focus when region changes
  useEffect(() => {
    if (selectedRegion && REGION_COORDINATES[selectedRegion]) {
      const [lat, long] = REGION_COORDINATES[selectedRegion];
      focusRef.current = locationToAngles(lat, long);
    }
  }, [selectedRegion]);

  useEffect(() => {
    if (!canvasRef.current) return;

    let windowWidth = 0;
    focusRef.current = locationToAngles(...REGION_COORDINATES["aws-us-east-1"]);
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
      scale: 1.15,
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
      markers: Object.values(REGION_COORDINATES).map(([lat, long]) => ({
        location: [lat, long],
        size: 0.07,
      })),
      onRender: (state) => {
        /* eslint-disable no-param-reassign */
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
        state.width = sm ? 1200 : 900;
        state.height = sm ? 1200 : 900;
        state.offset = sm ? [400, 900] : [500, -200];

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
  }, [isDark]);

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

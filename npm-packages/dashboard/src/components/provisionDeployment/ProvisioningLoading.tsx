import { Sheet } from "@ui/Sheet";

export function ProvisioningLoading({
  deploymentType,
}: {
  deploymentType: "prod" | "dev";
}) {
  const deploymentTypeLabel =
    deploymentType === "prod" ? "production" : "development";

  return (
    <div className="h-full bg-background-primary p-6">
      <Sheet className="mb-2 h-full overflow-hidden">
        <div className="flex flex-1 flex-col items-center justify-center">
          <div className="flex max-w-lg animate-fadeIn flex-col items-center">
            <h1 className="mx-2 mt-10 mb-8">
              Provisioning your{" "}
              <span className="font-semibold">{deploymentTypeLabel}</span>{" "}
              deployment...
            </h1>
            <div className="w-full animate-fadeIn">
              <div className="h-4 rounded-sm bg-background-tertiary" />
            </div>
          </div>
        </div>
      </Sheet>
    </div>
  );
}

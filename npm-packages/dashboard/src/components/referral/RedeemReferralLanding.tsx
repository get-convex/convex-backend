import { GitHubLogoIcon } from "@radix-ui/react-icons";
import { Button } from "dashboard-common/elements/Button";
import { Sheet } from "dashboard-common/elements/Sheet";
import { Spinner } from "dashboard-common/elements/Spinner";
import { cn } from "dashboard-common/lib/cn";
import { useState } from "react";

export function RedeemReferralLanding({
  title,
  code,
}: {
  title: string;
  code: string;
}) {
  return (
    <div className="relative mt-10 max-w-lg">
      <Sheet>
        <MarketingH1>{title}</MarketingH1>

        <MarketingP>
          Convex is the open-source reactive database for app developers.
        </MarketingP>

        <MarketingP>
          Accept this referral to double your free account quota.
        </MarketingP>

        <LogInButton code={code} />
      </Sheet>
    </div>
  );
}

function MarketingH1({ children }: React.PropsWithChildren) {
  return (
    <h1
      // eslint-disable-next-line no-restricted-syntax
      className="mb-6 font-marketing text-3xl font-black leading-[1.1] tracking-tight text-content-primary sm:text-5xl"
    >
      {children}
    </h1>
  );
}

function MarketingP({ children }: React.PropsWithChildren) {
  return (
    <p
      // eslint-disable-next-line no-restricted-syntax
      className="mb-4 text-xl font-medium leading-snug text-content-primary"
    >
      {children}
    </p>
  );
}

function LogInButton({ code }: { code: string }) {
  const [clicked, setClicked] = useState(false);

  return (
    <Button
      variant="unstyled"
      className={cn(
        "group z-10 inline-flex rounded-full bg-gradient-to-br from-[#8d2676_33%] via-[#ee342f] via-90% to-[#f3b01c] to-100% p-0.5 font-marketing shadow-[0_2px_14px_rgba(111,0,255,0.25)] transition-shadow my-2",
        !clicked && "hover:shadow-[rgba(111,0,255,0.5)]",
        clicked && "opacity-80 cursor-progress",
      )}
      href={`/api/auth/login?returnTo=/referral/${code}/apply`}
      onClick={() => setClicked(true)}
      aria-disabled={clicked}
    >
      <span
        // eslint-disable-next-line no-restricted-syntax
        className="flex w-full items-center gap-2 rounded-full bg-[#292929] px-8 py-2 text-center text-lg font-medium text-white transition-colors group-hover:bg-[#141414] lg:text-xl"
      >
        <GitHubLogoIcon className="size-6" />
        Sign up with GitHub
        {clicked && <Spinner className="size-4 text-white" />}
      </span>
    </Button>
  );
}

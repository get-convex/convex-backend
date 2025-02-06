import { ArrowRightIcon } from "@radix-ui/react-icons";
import classNames from "classnames";
import { buttonClasses } from "dashboard-common/elements/Button";

import { useRouter } from "next/router";

export function LoginWithEmail({ returnTo }: { returnTo?: string }) {
  const { query } = useRouter();
  return query.allowUsernameAuth || process.env.NODE_ENV !== "production" ? (
    <a
      href={`/api/auth/login?useEmail=true${
        returnTo ? `&returnTo=${returnTo}` : ""
      }`}
      className={classNames(
        "mx-auto mt-8 mb-2 z-20",
        buttonClasses({ size: "sm", variant: "neutral" }),
      )}
    >
      <span className="mr-2">Log in with Email</span>
      <ArrowRightIcon />
    </a>
  ) : null;
}

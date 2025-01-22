import { LoginPage } from "components/login/LoginPage";
import { LoginWithEmail } from "components/login/LoginWithEmail";
import { LoginLayout } from "layouts/LoginLayout";
import { useRouter } from "next/router";

function Signup() {
  const { query } = useRouter();
  const returnTo = query.returnTo
    ? query.returnTo.toString()
    : query.cta_plan_purchase_intent === "pro"
      ? "/settings/billing"
      : undefined;
  return (
    <div className="flex h-screen w-full flex-col items-center bg-background-brand">
      <LoginLayout>
        <LoginPage returnTo={returnTo} />
        <LoginWithEmail returnTo={returnTo} />
      </LoginLayout>
    </div>
  );
}

export default Signup;

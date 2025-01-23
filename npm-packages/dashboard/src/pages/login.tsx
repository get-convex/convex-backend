import { LoginPage } from "components/login/LoginPage";
import { LoginWithEmail } from "components/login/LoginWithEmail";
import { VHSLoginPage } from "components/login/VHSLoginPage";
import { Flourish, LoginLayout } from "layouts/LoginLayout";
import Head from "next/head";
import { useRouter } from "next/router";
import Background from "components/login/images/background.svg";

function Login() {
  const { query } = useRouter();
  const returnTo = query.returnTo
    ? query.returnTo.toString()
    : query.cta_plan_purchase_intent === "pro"
      ? "/settings/billing"
      : undefined;

  const vhsLoginPage = process.env.NEXT_PUBLIC_VHS_LOGIN_PAGE === "true";

  return (
    <>
      <Head>
        <link rel="canonical" href="https://dashboard.convex.dev/login" />
      </Head>
      <div className="flex h-screen w-full flex-col items-center bg-background-brand">
        {vhsLoginPage ? (
          <>
            <VHSLoginPage returnTo={returnTo} />
            <LoginWithEmail returnTo={returnTo} />
            <Flourish />
            <div className=" absolute left-1/2 top-24 hidden -translate-x-1/2 lg:block">
              <Background className="stroke-[#D7D7D7] dark:hidden" />
            </div>
          </>
        ) : (
          <LoginLayout>
            <LoginPage returnTo={returnTo} />
            <LoginWithEmail returnTo={returnTo} />
          </LoginLayout>
        )}
      </div>
    </>
  );
}

export default Login;

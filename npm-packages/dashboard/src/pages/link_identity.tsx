import { useEffect, useState, useRef, useLayoutEffect } from "react";
import { useRouter } from "next/router";
import { useAccessToken } from "hooks/useServerSideData";
import { withAuthenticatedPage } from "lib/withAuthenticatedPage";
import { useSessionStorage } from "react-use";
import { useLinkIdentity, useSetLinkIdentityCookie } from "api/profile";
import { useAuth0 } from "hooks/useAuth0";
import {
  LinkIdentityState,
  linkIdentityStateKey,
  providerToDisplayName,
} from "components/profile/ConnectedIdentities";
import { useLaunchDarkly } from "hooks/useLaunchDarkly";
import { LinkIdentityNoMultipleIdentities } from "components/profile/LinkIdentityNoMultipleIdentities";
import { LinkIdentitySuccessPrompt } from "components/profile/LinkIdentitySuccessPrompt";
import { LinkIdentityForm } from "components/profile/LinkIdentityForm";

export { getServerSideProps } from "lib/ssr";

function LinkIdentity() {
  const { multipleUserIdentities } = useLaunchDarkly();

  const { user } = useAuth0();
  const subParts = user?.sub?.split("|");
  const provider = subParts?.[0] === "oidc" ? subParts?.[1] : subParts?.[0];
  const providerDisplayName = provider
    ? providerToDisplayName[provider] || provider
    : undefined;
  const providerDisplayNameSafe = providerDisplayName || "";
  const providerSafe = provider || "";

  const {
    accessToken,
    resume,
    status,
    message,
    setLinkIdentityState,
    linkSuccess,
  } = useLinkIdentityStateMachine();

  // The feature flag is disabled, so show a UI that explains that the user
  // can't link multiple GitHub accounts -- they should contact us instead.
  if (!multipleUserIdentities) {
    return <LinkIdentityNoMultipleIdentities user={user} />;
  }

  // When we link an account from the profile page, we need to log in again
  // because the original JWT from the secondary account doesn't have the
  // primary account's subject associated with it.
  if (linkSuccess && resume === "fromProfile") {
    return <LinkIdentitySuccessPrompt />;
  }

  return (
    <LinkIdentityForm
      resume={resume}
      status={status}
      message={message}
      accessToken={accessToken}
      setLinkIdentityState={setLinkIdentityState}
      providerDisplayName={providerDisplayNameSafe}
      provider={providerSafe}
    />
  );
}

export default withAuthenticatedPage(LinkIdentity);

function useLinkIdentityStateMachine() {
  const [accessToken] = useAccessToken();
  const router = useRouter();
  const { resume } = router.query;
  const [status, setStatus] = useState<
    "waitingForCookie" | "ready" | "pending" | "error"
  >("waitingForCookie");
  const [message, setMessage] = useState<string>("");
  const [linkIdentityState, setLinkIdentityState] =
    useSessionStorage<LinkIdentityState>(linkIdentityStateKey, {});

  const [linkSuccess, setLinkSuccess] = useState(false);

  const setCookieInProgress = useRef(false);
  const setLinkIdentityCookie = useSetLinkIdentityCookie();
  useEffect(() => {
    void (async () => {
      if (
        router.isReady &&
        accessToken &&
        !resume &&
        status === "waitingForCookie" &&
        !setCookieInProgress.current
      ) {
        try {
          setCookieInProgress.current = true;
          await setLinkIdentityCookie();
        } catch {
          setStatus("error");
          setMessage("Failed to configure identity linking. Please try again.");
        }
        setStatus("ready");
      }
    })();
  }, [
    accessToken,
    resume,
    router.isReady,
    setLinkIdentityCookie,
    setLinkIdentityState,
    status,
    setCookieInProgress,
  ]);

  const linkIdentity = useLinkIdentity();
  const linkInProgress = useRef(false);
  const lastResumeRef = useRef<string | undefined>(undefined);
  const normalizedResume = typeof resume === "string" ? resume : undefined;

  useLayoutEffect(() => {
    void (async () => {
      if (
        !normalizedResume ||
        !accessToken ||
        status === "pending" ||
        status === "error" ||
        linkInProgress.current ||
        lastResumeRef.current === normalizedResume
      )
        return;
      linkInProgress.current = true;
      lastResumeRef.current = normalizedResume;
      setStatus("pending");
      setMessage("");
      try {
        await linkIdentity({ fromProfile: normalizedResume === "fromProfile" });
        if (normalizedResume === "fromProfile") {
          setLinkSuccess(true);
          setStatus("ready");
        } else {
          const destination = linkIdentityState.returnTo || "/profile";
          await router.push(destination);
        }
      } catch (err: any) {
        setStatus("error");
        setMessage(err.message || "Failed to link identity.");
      } finally {
        setLinkIdentityState({});
        linkInProgress.current = false;
      }
    })();
  }, [
    normalizedResume,
    accessToken,
    linkIdentityState,
    setLinkIdentityState,
    linkIdentity,
    status,
    router,
  ]);
  return {
    accessToken,
    resume: normalizedResume,
    status,
    message,
    setLinkIdentityState,
    linkSuccess,
  };
}

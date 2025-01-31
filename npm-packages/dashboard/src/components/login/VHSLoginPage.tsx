import Image from "next/image";
import classNames from "classnames";
import { useState } from "react";
import Logo from "images/convex-light.svg";
import { GoogleAnalytics } from "elements/GoogleAnalytics";
import loginBackground from "../../../public/vhs-bg.png";
import loading from "../../../public/vhs-loading.gif";
import GithubLogo from "./logos/github-logo.svg";

export function VHSLoginPage({ returnTo }: { returnTo?: string }) {
  const [isLoggingIn, setIsLoggingIn] = useState(false);
  const [isVHSAnimationDone, setIsVHSAnimationDone] = useState(false);
  return (
    <div>
      {/* Loading Google Analytics because we're not using the LoginLayout for this component. */}
      <GoogleAnalytics />
      <a
        className="group relative z-20 mt-36 flex w-[384px] flex-col items-center rounded-lg bg-neutral-8 focus:outline-none"
        style={{
          // Box shadow to make the TV look 3D.
          boxShadow: "32px 0px 0px 0px rgba(var(--neutral-7))",
        }}
        onClick={() => {
          setIsLoggingIn(true);
          setTimeout(() => setIsVHSAnimationDone(true), 1000);
        }}
        href={`/api/auth/login${returnTo ? `?returnTo=${returnTo}` : ""}`}
      >
        <TVPanel isVHSAnimationDone={isVHSAnimationDone} />
        <VHSTape isLoggingIn={isLoggingIn} />
      </a>
    </div>
  );
}

function VHSTape({ isLoggingIn }: { isLoggingIn: boolean }) {
  return (
    <div className="z-20 flex w-full flex-col items-center rounded-b-md px-16 pb-8 ">
      <div
        className={classNames(
          "relative transition-all duration-150 ease-in-out",
          "flex items-center gap-2 py-2 px-4 rounded bg-neutral-11",
          "-ml-10",
          !isLoggingIn && [
            // VHS Tape looks fairly ejected while not logging in.
            "shadow-[#393939_24px_0_1px_0,_black_24px_0_1px_0]",
            // While hovered or focused, VHS Tape looks like it's being pushed in.
            "group-hover:shadow-[#393939_12px_0_1px_0,_black_12px_0_1px_0]",
            "group-focus:shadow-[#393939_12px_0_1px_0,_black_12px_0_1px_0]",
            "group-hover:translate-x-3",
            "group-focus:translate-x-3",
            "group-focus:outline group-focus:outline-util-accent",
          ],
          isLoggingIn &&
            // If you're logging in, the VHS Tape should be pushed in all the way.
            "shadow-[#393939_0px_0px_1px_0,_black_0px_0px_1px_0] translate-x-5 cursor-not-allowed",
        )}
      >
        <div
          className={classNames(
            "absolute left-0 top-0 z-50  w-full rounded-sm bg-neutral-8 border border-neutral-11",
            isLoggingIn ? "block animate-vhs" : "hidden",
          )}
        />
        <div className="flex items-center">
          <div className="h-10 rounded-l-md bg-neutral-10 p-2">
            <GithubLogo className="fill-white" />
          </div>
          <div className="h-10 w-[13rem] bg-white px-8 py-2 font-semibold text-neutral-11">
            Log in with GitHub
          </div>
          <div className="h-10 w-1.5 bg-util-brand-purple" />
          <div className="h-10 w-1.5 bg-util-brand-red" />
          <div className="h-10 w-1.5 rounded-r-md bg-util-brand-yellow" />
        </div>
      </div>
    </div>
  );
}

function TVPanel({ isVHSAnimationDone }: { isVHSAnimationDone: boolean }) {
  return (
    <div
      className={classNames(
        "m-8 flex justify-center rounded-[20px] relative bg-neutral-11 z-20",
      )}
      style={{
        // Box shadow to make the TV panel look 3D.
        boxShadow: "-8px 0px 0px 0px rgba(var(--neutral-7))",
      }}
    >
      {isVHSAnimationDone ? (
        <>
          {/* eslint-disable-next-line no-restricted-syntax */}
          <div className="absolute left-[80px] top-[32px] z-50 font-mono text-xl font-semibold text-white">
            Logging in...
          </div>
          <Image
            src={loading.src}
            priority
            alt="loading gif"
            width={320}
            height={240}
            className="relative top-0 rounded-[20px] bg-neutral-11"
          />
        </>
      ) : (
        <>
          <Logo
            width={225}
            height={75}
            className="absolute left-[40px] top-[80px] z-50 fill-black dark:fill-black"
            alt="Convex Logo"
          />
          <Image
            src={loginBackground.src}
            alt="decorative background image"
            priority
            width={320}
            height={240}
            className="relative top-0 rounded-[20px] bg-neutral-11"
          />
        </>
      )}
    </div>
  );
}

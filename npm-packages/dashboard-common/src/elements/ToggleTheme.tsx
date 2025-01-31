import { SunIcon, MoonIcon, LightningBoltIcon } from "@radix-ui/react-icons";
import startCase from "lodash/startCase";
import { useTheme } from "next-themes";
import { cn } from "lib/cn";
import { Tooltip } from "./Tooltip";

export function ToggleTheme() {
  const { theme: currentTheme, setTheme } = useTheme();
  return (
    <div className="flex items-center justify-between gap-4 px-3 py-1">
      <span className="select-none">Theme</span>
      <fieldset className="flex items-center rounded-full border">
        <ThemeRadioInput
          currentTheme={currentTheme}
          setTheme={setTheme}
          theme="system"
          className="rounded-l-full"
        />
        <ThemeRadioInput
          currentTheme={currentTheme}
          setTheme={setTheme}
          theme="light"
        />
        <ThemeRadioInput
          currentTheme={currentTheme}
          setTheme={setTheme}
          theme="dark"
          className="rounded-r-full"
        />
      </fieldset>
    </div>
  );
}
function ThemeRadioInput({
  currentTheme,
  setTheme,
  className,
  theme,
}: {
  currentTheme?: string;
  setTheme: (theme: string) => void;
  className?: string;
  theme: string;
}) {
  const icon =
    theme === "light" ? (
      <SunIcon />
    ) : theme === "dark" ? (
      <MoonIcon />
    ) : (
      <LightningBoltIcon />
    );

  return (
    <>
      <input
        id={`${theme}-theme`}
        type="radio"
        onChange={() => setTheme(theme)}
        checked={!currentTheme || currentTheme === theme}
        hidden
      />
      <Tooltip tip={startCase(theme)} wrapsButton>
        <label
          aria-label="System Theme"
          htmlFor={`${theme}-theme`}
          className={cn(
            "p-1.5 cursor-pointer",
            currentTheme === theme
              ? "bg-util-accent text-white"
              : "hover:bg-background-tertiary",
            className,
          )}
        >
          {icon}
        </label>
      </Tooltip>
    </>
  );
}

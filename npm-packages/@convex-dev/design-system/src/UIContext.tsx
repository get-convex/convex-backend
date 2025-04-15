import { omit } from "lodash-es";
import {
  ComponentType,
  createContext,
  PropsWithChildren,
  useContext,
} from "react";
import { UrlObject } from "url";

type LinkProps = {
  href: string | UrlObject;
  className?: string;
  role?: string;
  tabIndex?: number;
  target?: string;
  onClick?: (event: React.MouseEvent<HTMLAnchorElement>) => void;
};

function encodeQuery(query: UrlObject["query"]): string {
  if (!query) return "";
  if (typeof query === "string") return `?${query}`;

  const params = new URLSearchParams();
  Object.entries(query).forEach(([key, value]) => {
    if (Array.isArray(value)) {
      value.forEach((v) => params.append(key, String(v)));
    } else if (value !== null && value !== undefined) {
      params.append(key, String(value));
    }
  });

  const searchString = params.toString();
  return searchString ? `?${searchString}` : "";
}

// Default link component that just renders an anchor tag
function DefaultLink({
  href,
  children,
  ...props
}: PropsWithChildren<LinkProps>) {
  return (
    <a
      href={
        typeof href === "string"
          ? href
          : `${href.pathname}${encodeQuery(href.query)}${href.hash || ""}`
      }
      {...omit(props, "passHref")}
    >
      {children}
    </a>
  );
}

// Create context with the default link component
export const UIContext =
  createContext<ComponentType<PropsWithChildren<LinkProps>>>(DefaultLink);

type UIProviderProps = PropsWithChildren<{
  Link?: ComponentType<PropsWithChildren<LinkProps>>;
}>;

function UIProvider({ children, Link }: UIProviderProps) {
  return (
    <UIContext.Provider value={Link ?? DefaultLink}>
      {children}
    </UIContext.Provider>
  );
}

export { UIProvider };

// Custom hook for using the UI context
export function useUIContext() {
  return useContext(UIContext);
}

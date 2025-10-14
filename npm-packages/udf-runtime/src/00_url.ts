import { throwNotImplementedMethodError } from "./helpers.js";
import { performOp } from "udf-syscall-ffi";
import inspect from "object-inspect";

type Update =
  | {
      hash: string | null;
    }
  | {
      hostname: string | null;
    }
  | {
      href: string;
    }
  | {
      port: string | null;
    }
  | {
      protocol: string;
    }
  | {
      pathname: string;
    }
  | {
      search: string | null;
    }
  | {
      searchParams: [string, string][];
    };

// Private symbols for URL to poke at the internals of URLSearchParams
const _searchParamPairs = Symbol("_searchParamPairs");
const _urlObjectUpdate = Symbol("_urlObjectUpdate");

class URLSearchParams {
  [_searchParamPairs]: [string, string][];
  // Reference back to the parent URL's `#updateUrl` method
  [_urlObjectUpdate]?: (update: { searchParams: [string, string][] }) => void;
  constructor(
    init?: string[][] | Record<string, string> | string | URLSearchParams,
  ) {
    this[_searchParamPairs] = [];
    if (init === undefined) {
      this[_searchParamPairs] = [];
    } else if (typeof init === "string") {
      const queryString = init.startsWith("?") ? init.slice(1) : init;
      this[_searchParamPairs] = performOp(
        "url/getUrlSearchParamPairs",
        queryString,
      );
    } else if (Array.isArray(init)) {
      for (const pair of init) {
        if (pair.length !== 2) {
          throw new TypeError(
            "Failed to construct 'URLSearchParams': Failed to construct 'URLSearchParams': Sequence initializer must only contain pair elements",
          );
        }
        this.append(pair[0], pair[1]);
      }
    } else if (init instanceof URLSearchParams) {
      init.forEach((value, key) => {
        this.append(key, value);
      });
    } else {
      for (const key in init) {
        this.append(key, init[key]);
      }
    }
  }

  private _updateUrl() {
    if (this[_urlObjectUpdate] !== undefined) {
      this[_urlObjectUpdate]({
        searchParams: this[_searchParamPairs],
      });
    }
  }

  append(name: string, value: string): void {
    this[_searchParamPairs].push([String(name), String(value)]);
    this._updateUrl();
  }

  delete(name: string) {
    this[_searchParamPairs] = this[_searchParamPairs].filter(([key]) => {
      return key !== String(name);
    });
    this._updateUrl();
  }

  entries(): IterableIterator<[string, string]> {
    return this[_searchParamPairs][Symbol.iterator]();
  }

  forEach(
    callbackFn: (value: string, key: string, parent: URLSearchParams) => void,
  ) {
    this[_searchParamPairs].forEach(([key, value]) => {
      callbackFn(value, key, this);
    });
  }

  get(name: string): string | null {
    return this.getAll(String(name))[0] ?? null;
  }

  getAll(name: string): string[] {
    const values: string[] = [];
    for (const [key, value] of this[_searchParamPairs]) {
      if (key === name) {
        values.push(value);
      }
    }
    return values;
  }

  has(name: string): boolean {
    return (
      this[_searchParamPairs].find(([key]) => key === String(name)) !==
      undefined
    );
  }

  keys(): IterableIterator<string> {
    return this[_searchParamPairs].map(([key]) => key)[Symbol.iterator]();
  }

  set(name: string, value: string) {
    this.delete(name);
    this.append(name, value);
    this._updateUrl();
  }

  sort() {
    this[_searchParamPairs].sort((a, b) => {
      return a[0].localeCompare(b[0]);
    });
    this._updateUrl();
  }

  toString() {
    return performOp("url/stringifyUrlSearchParams", this[_searchParamPairs]);
  }

  toJSON() {
    return {};
  }

  [Symbol.iterator](): IterableIterator<[string, string]> {
    return this.entries();
  }

  values(): IterableIterator<string> {
    return this[_searchParamPairs].map(([, value]) => value)[Symbol.iterator]();
  }

  get [Symbol.toStringTag]() {
    return "URLSearchParams";
  }

  inspect() {
    let inner = "";
    if (this[_searchParamPairs].length !== 0) {
      inner =
        " " +
        this[_searchParamPairs]
          .map(([k, v]) => `${inspect(k)} => ${inspect(v)}`)
          .join(", ") +
        " ";
    }
    return `${this.constructor.name} {${inner}}`;
  }
}

type UrlInfo = {
  scheme: string;
  hash: string;
  host: string;
  hostname: string;
  pathname: string;
  port: string;
  search: string;
  href: string;
  username: string;
  password: string;
};

type URLInfoResult =
  | { kind: "success"; urlInfo: UrlInfo }
  | {
      kind: "error";
      errorType: "UnsupportedURL" | "InvalidURL";
      message?: string;
    };

class URL {
  #urlInfo: UrlInfo;
  #searchParams: URLSearchParams;

  constructor(url: string | URL, base?: string | URL) {
    let baseHref: string | null = null;
    if (base !== undefined) {
      baseHref = typeof base === "string" ? base : base.href;
    }
    if (typeof url === "string") {
      const urlInfoResult: URLInfoResult = performOp(
        "url/getUrlInfo",
        url,
        baseHref,
      );
      if (urlInfoResult.kind === "error") {
        switch (urlInfoResult.errorType) {
          case "InvalidURL":
            throw new TypeError(`Invalid URL: '${url}'`);
          case "UnsupportedURL":
            throw new TypeError(urlInfoResult.message ?? "Unsupported URL");
        }
      }
      this.#urlInfo = urlInfoResult.urlInfo;
    } else {
      this.#urlInfo = { ...url.#urlInfo };
    }
    this.#searchParams = new URLSearchParams(this.#urlInfo.search ?? "");
    this.#searchParams[_urlObjectUpdate] = this.#updateUrl.bind(this);
  }

  get hash() {
    return this.#urlInfo.hash !== "" ? `#${this.#urlInfo.hash}` : "";
  }

  set hash(_hash: string) {
    let newHash: string | null = _hash.startsWith("#") ? _hash.slice(1) : _hash;
    newHash = newHash === "" ? null : newHash;
    this.#updateUrl({
      hash: newHash,
    });
  }

  get host() {
    return this.#urlInfo.host;
  }

  set host(_host: string) {
    throwNotImplementedMethodError("set host", "URL");
  }

  get hostname() {
    return this.#urlInfo.hostname;
  }

  set hostname(_hostname: string) {
    this.#updateUrl({
      hostname: _hostname === "" ? null : _hostname,
    });
  }

  get href() {
    return this.#urlInfo.href;
  }

  set href(_href: string) {
    const urlInfo = performOp("url/getUrlInfo", _href);
    if (urlInfo === null) {
      throw new TypeError(
        "Failed to set the 'href' property on 'URL': Invalid URL",
      );
    }
    this.#updateUrl({
      href: _href,
    });
  }

  get origin() {
    switch (this.#urlInfo.scheme) {
      case "ftp":
      case "http":
      case "https":
      case "ws":
      case "wss":
        return `${this.#urlInfo.scheme}://${this.host}`;
      default:
        return "null";
    }
  }

  get password() {
    return this.#urlInfo.password;
  }

  set password(_password: string) {
    throwNotImplementedMethodError("set password", "URL");
  }

  get pathname() {
    return this.#urlInfo.pathname;
  }

  set pathname(_pathname: string) {
    this.#updateUrl({
      pathname: _pathname,
    });
  }

  get port() {
    return this.#urlInfo.port?.toString() ?? "";
  }

  set port(_port: string) {
    this.#updateUrl({
      port: _port === "" ? null : _port,
    });
  }

  get protocol() {
    return this.#urlInfo.scheme.toString() + ":";
  }

  set protocol(_protocol: string) {
    this.#updateUrl({
      protocol: _protocol,
    });
  }

  get search() {
    return this.#urlInfo.search === "" ? "" : `?${this.#urlInfo.search}`;
  }

  set search(_search: string) {
    let newSearch: string | null = _search.startsWith("?")
      ? _search.slice(1)
      : _search;
    newSearch = newSearch === "" ? null : newSearch;
    this.#updateUrl({
      search: newSearch,
    });
  }

  get searchParams() {
    return this.#searchParams;
  }

  get username() {
    return this.#urlInfo.username;
  }

  set username(_username: string) {
    throwNotImplementedMethodError("set username", "URL");
  }

  toString() {
    return this.href;
  }

  toJSON() {
    return this.href;
  }

  #updateUrl(update: Update) {
    this.#urlInfo = performOp("url/updateUrlInfo", this.href, update);
    // Mutate the existing searchParams object
    const searchPairs = performOp(
      "url/getUrlSearchParamPairs",
      this.#urlInfo.search,
    );
    this.#searchParams[_searchParamPairs] = searchPairs;
  }

  get [Symbol.toStringTag]() {
    return "URL";
  }

  inspect() {
    const object = {
      href: this.href,
      origin: this.origin,
      protocol: this.protocol,
      username: this.username,
      password: this.password,
      host: this.host,
      hostname: this.hostname,
      port: this.port,
      pathname: this.pathname,
      hash: this.hash,
      search: this.search,
    };
    return `${this.constructor.name} ${inspect(object)}`;
  }
}

export const setupURL = (global: any) => {
  global.URL = URL;
  global.URLSearchParams = URLSearchParams;
};

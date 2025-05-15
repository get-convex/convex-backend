import { throwNotImplementedMethodError } from "./helpers.js";
import { performOp } from "udf-syscall-ffi";
import inspect from "object-inspect";

type Update =
  | {
      type: "hash";
      value: string | null;
    }
  | {
      type: "hostname";
      value: string | null;
    }
  | {
      type: "href";
      value: string;
    }
  | {
      type: "port";
      value: string | null;
    }
  | {
      type: "protocol";
      value: string;
    }
  | {
      type: "pathname";
      value: string;
    }
  | {
      type: "search";
      value: string | null;
    }
  | {
      type: "searchParams";
      value: [string, string][];
    };

class URLSearchParams {
  private _searchParamPairs: [string, string][];
  private _urlObject: URL | null = null;
  constructor(
    init?: string[][] | Record<string, string> | string | URLSearchParams,
  ) {
    this._searchParamPairs = [];
    if (init === undefined) {
      this._searchParamPairs = [];
    } else if (typeof init === "string") {
      const queryString = init.startsWith("?") ? init.slice(1) : init;
      this._searchParamPairs = performOp(
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
    if (this._urlObject !== null) {
      this._urlObject.updateUrl({
        type: "searchParams",
        value: this._searchParamPairs,
      });
    }
  }

  append(name: string, value: string): void {
    this._searchParamPairs.push([String(name), String(value)]);
    this._updateUrl();
  }

  delete(name: string) {
    this._searchParamPairs = this._searchParamPairs.filter(([key]) => {
      return key !== String(name);
    });
    this._updateUrl();
  }

  entries(): IterableIterator<[string, string]> {
    return this._searchParamPairs[Symbol.iterator]();
  }

  forEach(
    callbackFn: (value: string, key: string, parent: URLSearchParams) => void,
  ) {
    this._searchParamPairs.forEach(([key, value]) => {
      callbackFn(value, key, this);
    });
  }

  get(name: string): string | null {
    return this.getAll(String(name))[0] ?? null;
  }

  getAll(name: string): string[] {
    const values: string[] = [];
    for (const [key, value] of this._searchParamPairs) {
      if (key === name) {
        values.push(value);
      }
    }
    return values;
  }

  has(name: string): boolean {
    return (
      this._searchParamPairs.find(([key]) => key === String(name)) !== undefined
    );
  }

  keys(): IterableIterator<string> {
    return this._searchParamPairs.map(([key]) => key)[Symbol.iterator]();
  }

  set(name: string, value: string) {
    this.delete(name);
    this.append(name, value);
    this._updateUrl();
  }

  sort() {
    this._searchParamPairs.sort((a, b) => {
      return a[0].localeCompare(b[0]);
    });
    this._updateUrl();
  }

  toString() {
    return performOp("url/stringifyUrlSearchParams", this._searchParamPairs);
  }

  toJSON() {
    return {};
  }

  [Symbol.iterator](): IterableIterator<[string, string]> {
    return this.entries();
  }

  values(): IterableIterator<string> {
    return this._searchParamPairs.map(([, value]) => value)[Symbol.iterator]();
  }
}

type UrlInfo = {
  scheme: string;
  hash: string;
  host: string;
  hostname: string;
  pathname: string;
  port: string;
  protocol: string;
  search: string;
  href: string;
};

type URLInfoResult =
  | { kind: "success"; urlInfo: UrlInfo }
  | {
      kind: "error";
      errorType: "UnsupportedURL" | "InvalidURL";
      message?: string;
    };

class URL {
  private _urlInfo: UrlInfo;
  private _searchParams: URLSearchParams;

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
      this._urlInfo = urlInfoResult.urlInfo;
    } else {
      this._urlInfo = { ...url._urlInfo };
    }
    this._searchParams = new URLSearchParams(this._urlInfo.search ?? "");
    (this._searchParams as any)._urlObject = this;
  }

  get hash() {
    return this._urlInfo.hash !== "" ? `#${this._urlInfo.hash}` : "";
  }

  set hash(_hash: string) {
    let newHash: string | null = _hash.startsWith("#") ? _hash.slice(1) : _hash;
    newHash = newHash === "" ? null : newHash;
    this.updateUrl({
      type: "hash",
      value: newHash,
    });
  }

  get host() {
    return this._urlInfo.host;
  }

  set host(_host: string) {
    throwNotImplementedMethodError("set host", "URL");
  }

  get hostname() {
    return this._urlInfo.hostname;
  }

  set hostname(_hostname: string) {
    this.updateUrl({
      type: "hostname",
      value: _hostname === "" ? null : _hostname,
    });
  }

  get href() {
    return this._urlInfo.href;
  }

  set href(_href: string) {
    const urlInfo = performOp("url/getUrlInfo", _href);
    if (urlInfo === null) {
      throw new TypeError(
        "Failed to set the 'href' property on 'URL': Invalid URL",
      );
    }
    this.updateUrl({
      type: "href",
      value: _href,
    });
  }

  get origin() {
    return `${this.protocol}//${this.host}`;
  }

  get password() {
    return throwNotImplementedMethodError("get password", "URL");
  }

  set password(_password: string) {
    throwNotImplementedMethodError("set password", "URL");
  }

  get pathname() {
    return this._urlInfo.pathname;
  }

  set pathname(_pathname: string) {
    this.updateUrl({
      type: "pathname",
      value: _pathname,
    });
  }

  get port() {
    return this._urlInfo.port?.toString() ?? "";
  }

  set port(_port: string) {
    this.updateUrl({
      type: "port",
      value: _port === "" ? null : _port,
    });
  }

  get protocol() {
    return this._urlInfo.protocol.toString() + ":";
  }

  set protocol(_protocol: string) {
    this.updateUrl({
      type: "protocol",
      value: _protocol,
    });
  }

  get search() {
    return this._urlInfo.search === "" ? "" : `?${this._urlInfo.search}`;
  }

  set search(_search: string) {
    let newSearch: string | null = _search.startsWith("?")
      ? _search.slice(1)
      : _search;
    newSearch = newSearch === "" ? null : newSearch;
    this.updateUrl({
      type: "search",
      value: newSearch,
    });
  }

  get searchParams() {
    return this._searchParams;
  }

  get username() {
    return throwNotImplementedMethodError("get username", "URL");
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

  updateUrl(update: Update) {
    this._urlInfo = performOp("url/updateUrlInfo", this.href, update);
    // Mutate the existing searchParams object
    const searchPairs = performOp(
      "url/getUrlSearchParamPairs",
      this._urlInfo.search,
    );
    (this._searchParams as any)._searchParamPairs = searchPairs;
  }

  get [Symbol.toStringTag]() {
    return "URL";
  }

  inspect() {
    const object = {
      href: this.href,
      origin: this.origin,
      protocol: this.protocol,
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

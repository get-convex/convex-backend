import { throwNotImplementedMethodError } from "./helpers.js";
import { Blob, isSupportedBlobPart } from "./09_file.js";
import { parseFormData, FormData, formDataToBlob } from "./21_formdata.js";
import inspect from "object-inspect";
import {
  constructStreamId,
  extractStream,
  ReadableStream,
} from "./06_streams.js";

const _contentLength = Symbol("[[contentLength]]");
export const _redirected = Symbol("[[redirected]]");
const _responseType = Symbol("[[responseType]]");

export class Response {
  private _status: number;
  private _statusText: string;
  private _headers: Headers;
  private _bodyStream: ReadableStream | null;
  private _bodyUsed = false;
  private _url: string;
  [_contentLength]: number | null;
  [_redirected]: boolean;
  [_responseType]: ResponseType;

  static error() {
    return new Response(null, { status: 500 });
  }

  static redirect(url: string | URL, status?: number) {
    const location = typeof url === "string" ? url : url.href;

    return new Response(null, {
      status: status ?? 302,
      headers: new Headers({ location }),
    });
  }

  static json(
    data: any,
    init?: {
      status?: number;
      statusText?: string;
      headers?: [string, string][] | Headers | Record<string, string>;
      url?: string;
    },
  ) {
    let body = "";
    if (data === undefined) {
      throw new TypeError(
        "Failed to execute 'json' on 'Response': The data is not JSON serializable",
      );
    }
    try {
      body = JSON.stringify(data);
    } catch (e) {
      throw new TypeError(
        "Failed to execute 'json' on 'Response': The data is not JSON serializable",
      );
    }
    const res = new Response(body, init);
    res.headers.set("content-type", "application/json");
    return res;
  }

  constructor(
    body?:
      | string
      | ArrayBuffer
      | null
      | Blob
      | ArrayBufferView
      | URLSearchParams
      | ReadableStream
      | FormData,
    options?: {
      status?: number;
      statusText?: string;
      headers?: [string, string][] | Headers | Record<string, string>;
      url?: string;
    },
  ) {
    let status = options?.status === undefined ? 200 : options.status;
    if (typeof status === "string") {
      // This coerces the string to a number (and is different from `parseInt` which allows trailing characters after a valid number)
      status = +status;
    }
    if (
      typeof status !== "number" ||
      Number.isNaN(status) ||
      !Number.isInteger(status) ||
      status < 200 ||
      status > 599
    ) {
      throw new RangeError(
        "Failed to construct 'Response': The status provided is outside the range [200, 599].",
      );
    }
    this._status = status;
    this._statusText = options?.statusText ?? "";
    this._headers = new Headers(options?.headers ?? []);
    this._url = options?.url ?? "";
    this[_contentLength] = null;
    this[_redirected] = false;
    this[_responseType] = "default";

    if (this._headers.get("content-length") !== null) {
      this[_contentLength] = Number(this._headers.get("content-length"));
    }

    // Fill in a content type if none was provided and the body is a string
    if (this._headers.get("content-type") === null) {
      if (typeof body === "string") {
        this._headers.set("content-type", "text/plain;charset=UTF-8");
      } else if (body instanceof Blob && body.type !== "") {
        this._headers.set("content-type", body.type);
      }
    }

    if (body !== null && body !== undefined) {
      if (body instanceof URLSearchParams) {
        body = body.toString();
      }
      if (body instanceof FormData) {
        const bodyBlob = formDataToBlob(body);
        this._headers.set("content-type", bodyBlob.type);
        this[_contentLength] = bodyBlob.size;
        this._bodyStream = bodyBlob.stream();
      } else if (body instanceof ReadableStream) {
        this._bodyStream = body;
      } else if (isSupportedBlobPart(body)) {
        const bodyBlob = new Blob([body], {
          type: this._headers.get("content-type") ?? undefined,
        });
        this[_contentLength] = bodyBlob.size;
        this._bodyStream = bodyBlob.stream();
      } else {
        return throwNotImplementedMethodError(
          "constructor with body type other than string | ArrayBuffer | Blob | ReadableStream | null",
          "Response",
        );
      }
    } else {
      this._bodyStream = null;
    }
  }

  private _markBodyUsed(method: string) {
    if (this._bodyUsed) {
      throw new TypeError(
        `Failed to execute '${method}' on 'Response': body stream already read`,
      );
    }
    // Apparently, using the body multiple times if there is no body is fine.
    if (this._bodyStream !== null) {
      this._bodyUsed = true;
    }
  }

  private async _blob() {
    if (this._bodyStream === null) {
      return Promise.resolve(new Blob());
    }
    const type = this.headers.get("content-type") ?? "";
    if (this[_contentLength] !== null) {
      return Blob.fromStream(this._bodyStream, this[_contentLength], type);
    }
    const reader = this._bodyStream.getReader();
    const chunks: Uint8Array[] = [];
    const read = async (): Promise<void> => {
      const { done, value } = await reader.read();
      if (!done && value) {
        chunks.push(value);
        return read();
      }
    };
    await read();
    return new Blob(chunks, { type });
  }

  get bodyUsed() {
    return this._bodyUsed;
  }

  get headers() {
    return this._headers;
  }

  get ok() {
    return this._status >= 200 && this._status <= 299;
  }

  get status() {
    return this._status;
  }

  get statusText() {
    return this._statusText;
  }

  async blob(): Promise<Blob> {
    this._markBodyUsed("blob");
    const blob = await this._blob();
    return blob;
  }

  async text(): Promise<string> {
    this._markBodyUsed("text");
    const blob = await this._blob();
    return blob.text();
  }

  async json(): Promise<any> {
    this._markBodyUsed("json");
    const blob = await this._blob();
    const text = await blob.text();
    return JSON.parse(text);
  }

  async arrayBuffer(): Promise<ArrayBuffer> {
    this._markBodyUsed("arrayBuffer");
    const blob = await this._blob();
    return blob.arrayBuffer();
  }

  async formData(): Promise<FormData> {
    this._markBodyUsed("formData");
    const blob = await this._blob();
    return parseFormData(blob, this._headers.get("content-type"));
  }

  get body() {
    return this._bodyStream;
  }

  clone() {
    const clonedHeaderPairs: [string, string][] = [];
    this._headers.forEach((headerValue, headerName) =>
      clonedHeaderPairs.push([headerName, headerValue]),
    );

    const [bodyStreamA, bodyStreamB] =
      this._bodyStream !== null ? this._bodyStream.tee() : [null, null];
    const clonedResponse = new Response(null, {
      status: this._status,
      statusText: this._statusText,
      headers: clonedHeaderPairs,
    });
    clonedResponse._bodyStream = bodyStreamB;
    this._bodyStream = bodyStreamA;
    return clonedResponse;
  }

  get url(): string {
    return this._url;
  }

  get type() {
    // TODO: There are more types, but we haven't implemented any of the
    // functionality that would result in them
    // https://developer.mozilla.org/en-US/docs/Web/API/Response/type.
    return this[_responseType];
  }

  inspect() {
    const properties = {
      bodyUsed: this._bodyUsed,
      headers: this._headers,
      ok: this.ok,
      status: this.status,
      statusText: this.statusText,
      url: this._url,
    };
    return `Request ${inspect(properties)}`;
  }

  // ---------------------------------------------------------------
  // Begin unimplemented functions
  // ---------------------------------------------------------------

  get redirected() {
    return this[_redirected];
  }
}

export const convexJsonFromResponse = ({
  response,
}: {
  response: Response;
}) => {
  const streamId = constructStreamId(response.body);
  const headerPairs = [...response.headers.entries()];
  if (
    response[_contentLength] !== null &&
    !response.headers.has("content-length")
  ) {
    headerPairs.push(["content-length", String(response[_contentLength])]);
  }
  return {
    headerPairs,
    status: response.status,
    streamId,
    url: response.url !== "" ? response.url : undefined,
  };
};

export const responseFromConvexObject = (convexObject: Record<string, any>) => {
  const body = convexObject.streamId
    ? extractStream(convexObject.streamId)
    : null;
  const response = new Response(body, {
    status: Number(convexObject.status),
    statusText: convexObject.statusText,
    headers: convexObject.headerPairs,
    url: convexObject.url,
  });
  response[_responseType] = "basic";
  return response;
};

export const setupResponse = (global: any) => {
  global.Response = Response;
};

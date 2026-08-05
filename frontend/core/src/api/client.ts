/**
 * TR-07-002 — Typed API client.
 *
 * Parses the house success envelope and returns the typed `data`; on any
 * non-2xx it parses the RFC 9457 problem and throws {@link ApiError}. The
 * client is transport-injectable (a `fetch` impl) and token-injectable (a
 * `getToken` callback) so it is fully unit-testable and drives TR-07-003's
 * Authorization-header injection and refresh-on-401 behaviour.
 */
import { ApiError, PROBLEM_JSON, toProblem } from "./problem";
import type { SuccessEnvelope } from "./types";

export type FetchLike = (
  input: string,
  init?: RequestInit,
) => Promise<Response>;

export interface ApiClientOptions {
  baseUrl: string;
  /** Injected transport; defaults to the global `fetch`. */
  fetchImpl?: FetchLike;
  /** Returns the current access token (or null when unauthenticated). */
  getToken?: () => string | null;
  /**
   * Invoked on a 401 before a single retry. Return `true` if a fresh token is
   * now available (retry proceeds); `false` to surface the 401 to the caller.
   */
  onUnauthorized?: () => Promise<boolean>;
}

interface RequestOptions {
  method?: string;
  body?: unknown;
  /** Extra headers (rarely needed). */
  headers?: Record<string, string>;
  /** Internal: prevents infinite refresh loops. */
  _isRetry?: boolean;
  signal?: AbortSignal;
}

export class ApiClient {
  private readonly baseUrl: string;
  private readonly fetchImpl: FetchLike;
  private readonly getToken: () => string | null;
  private readonly onUnauthorized?: () => Promise<boolean>;

  constructor(opts: ApiClientOptions) {
    this.baseUrl = opts.baseUrl.replace(/\/+$/, "");
    this.fetchImpl =
      opts.fetchImpl ?? ((input, init) => fetch(input, init));
    this.getToken = opts.getToken ?? (() => null);
    this.onUnauthorized = opts.onUnauthorized;
  }

  async request<T>(path: string, options: RequestOptions = {}): Promise<T> {
    const headers: Record<string, string> = {
      Accept: "application/json",
      ...options.headers,
    };
    if (options.body !== undefined) {
      headers["Content-Type"] = "application/json";
    }
    const token = this.getToken();
    if (token) {
      headers["Authorization"] = `Bearer ${token}`;
    }

    const url = path.startsWith("http")
      ? path
      : `${this.baseUrl}${path.startsWith("/") ? path : `/${path}`}`;

    const res = await this.fetchImpl(url, {
      method: options.method ?? "GET",
      headers,
      body: options.body !== undefined ? JSON.stringify(options.body) : undefined,
      signal: options.signal,
    });

    // 204 / empty success.
    if (res.status === 204) {
      return undefined as T;
    }

    const contentType = res.headers.get("content-type") ?? "";
    const isProblem = contentType.includes(PROBLEM_JSON);

    if (!res.ok || isProblem) {
      // Attempt a single refresh-and-retry on 401.
      if (
        res.status === 401 &&
        !options._isRetry &&
        this.onUnauthorized &&
        (await this.onUnauthorized())
      ) {
        return this.request<T>(path, { ...options, _isRetry: true });
      }
      const body = await this.safeJson(res);
      throw new ApiError(toProblem(body, res.status));
    }

    const envelope = (await res.json()) as SuccessEnvelope<T>;
    return envelope.data;
  }

  get<T>(path: string, options?: RequestOptions): Promise<T> {
    return this.request<T>(path, { ...options, method: "GET" });
  }

  post<T>(path: string, body?: unknown, options?: RequestOptions): Promise<T> {
    return this.request<T>(path, { ...options, method: "POST", body });
  }

  put<T>(path: string, body?: unknown, options?: RequestOptions): Promise<T> {
    return this.request<T>(path, { ...options, method: "PUT", body });
  }

  del<T>(path: string, options?: RequestOptions): Promise<T> {
    return this.request<T>(path, { ...options, method: "DELETE" });
  }

  private async safeJson(res: Response): Promise<unknown> {
    try {
      return await res.json();
    } catch {
      return undefined;
    }
  }
}

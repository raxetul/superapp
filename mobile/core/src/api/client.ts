/**
 * Typed API client (TR-08-002).
 *
 * Parses the house success envelope and unwraps `data`; surfaces RFC 9457
 * Problem Details as a typed {@link ApiError}. On a `401` it invokes an
 * optional refresh hook exactly once and retries, enabling transparent token
 * renewal (TR-08-003).
 */
import {
  isProblemDetails,
  isSuccessEnvelope,
  type FieldError,
  type Pagination,
  type ProblemDetails,
  type SuccessEnvelope,
} from './types';

/** Error carrying a parsed RFC 9457 Problem Details document. */
export class ApiError extends Error {
  constructor(
    public readonly problem: ProblemDetails,
    public readonly httpStatus: number,
  ) {
    super(problem.detail ?? problem.title);
    this.name = 'ApiError';
  }

  /** Field-level validation failures, if any. */
  get fieldErrors(): FieldError[] {
    return this.problem.errors ?? [];
  }
}

/** Supplies (and, on demand, refreshes) the bearer access token. */
export interface TokenProvider {
  getAccessToken(): Promise<string | null>;
  /** Refresh on 401; resolves with a new access token or null if impossible. */
  refresh?(): Promise<string | null>;
}

export interface ApiClientOptions {
  baseUrl: string;
  tokenProvider?: TokenProvider;
  /** Injectable fetch (defaults to global `fetch`), for testing. */
  fetchImpl?: typeof fetch;
}

export interface RequestOptions {
  /** JSON body; serialized automatically. */
  body?: unknown;
  /** Attach the bearer token (default true). */
  auth?: boolean;
  /** Extra headers. */
  headers?: Record<string, string>;
  signal?: AbortSignal;
}

type Method = 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE';

function problemFromStatus(status: number, statusText: string): ProblemDetails {
  return {
    type: 'about:blank',
    title: statusText || `HTTP ${status}`,
    status,
  };
}

export class ApiClient {
  private readonly baseUrl: string;
  private readonly tokenProvider?: TokenProvider;
  private readonly fetchImpl: typeof fetch;

  constructor(options: ApiClientOptions) {
    this.baseUrl = options.baseUrl.replace(/\/+$/, '');
    this.tokenProvider = options.tokenProvider;
    this.fetchImpl = options.fetchImpl ?? globalThis.fetch;
  }

  /** Perform a request and return the unwrapped `data` from the envelope. */
  async request<T>(method: Method, path: string, options: RequestOptions = {}): Promise<T> {
    const envelope = await this.requestEnvelope<T>(method, path, options);
    return envelope.data;
  }

  /** Perform a request and return the full success envelope (with pagination). */
  async requestEnvelope<T>(
    method: Method,
    path: string,
    options: RequestOptions = {},
  ): Promise<SuccessEnvelope<T>> {
    const res = await this.send(method, path, options, false);
    return this.parse<T>(res);
  }

  private async send(
    method: Method,
    path: string,
    options: RequestOptions,
    isRetry: boolean,
  ): Promise<Response> {
    const useAuth = options.auth !== false;
    const headers: Record<string, string> = {
      Accept: 'application/json, application/problem+json',
      ...options.headers,
    };

    if (options.body !== undefined) {
      headers['Content-Type'] = 'application/json';
    }

    if (useAuth && this.tokenProvider) {
      const token = await this.tokenProvider.getAccessToken();
      if (token) headers.Authorization = `Bearer ${token}`;
    }

    const url = `${this.baseUrl}${path.startsWith('/') ? path : `/${path}`}`;
    const res = await this.fetchImpl(url, {
      method,
      headers,
      body: options.body !== undefined ? JSON.stringify(options.body) : undefined,
      signal: options.signal,
    });

    // Transparent single-shot refresh on 401 (TR-08-003).
    if (res.status === 401 && useAuth && !isRetry && this.tokenProvider?.refresh) {
      const refreshed = await this.tokenProvider.refresh();
      if (refreshed) {
        return this.send(method, path, options, true);
      }
    }

    return res;
  }

  private async parse<T>(res: Response): Promise<SuccessEnvelope<T>> {
    const text = await res.text();
    let json: unknown;
    try {
      json = text.length > 0 ? JSON.parse(text) : undefined;
    } catch {
      json = undefined;
    }

    if (res.ok) {
      if (isSuccessEnvelope(json)) {
        return json as SuccessEnvelope<T>;
      }
      // 2xx without the envelope (e.g. 204/empty) — synthesize one.
      return { success: true, data: (json ?? null) as T };
    }

    const problem = isProblemDetails(json)
      ? json
      : problemFromStatus(res.status, res.statusText);
    throw new ApiError(problem, res.status);
  }

  get<T>(path: string, options?: RequestOptions): Promise<T> {
    return this.request<T>('GET', path, options);
  }

  post<T>(path: string, body?: unknown, options?: RequestOptions): Promise<T> {
    return this.request<T>('POST', path, { ...options, body });
  }

  put<T>(path: string, body?: unknown, options?: RequestOptions): Promise<T> {
    return this.request<T>('PUT', path, { ...options, body });
  }

  delete<T>(path: string, options?: RequestOptions): Promise<T> {
    return this.request<T>('DELETE', path, options);
  }
}

export type { Pagination };

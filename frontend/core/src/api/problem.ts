/**
 * TR-07-002 — RFC 9457 Problem Details surfaced to callers.
 *
 * Error responses arrive as `application/problem+json`. We parse them into a
 * typed {@link Problem} and raise an {@link ApiError} so every caller handles
 * failures uniformly (never by inspecting raw JSON).
 */

/** RFC 6901 JSON-pointer field error carried in `Problem.errors`. */
export interface FieldError {
  pointer: string;
  detail: string;
}

/** An RFC 9457 Problem Details document. */
export interface Problem {
  type: string;
  title: string;
  status: number;
  detail?: string;
  instance?: string;
  errors?: FieldError[];
  /** RFC 9457 permits arbitrary extension members. */
  [key: string]: unknown;
}

export const PROBLEM_JSON = "application/problem+json";

/** Error thrown for any non-2xx API response, wrapping the parsed problem. */
export class ApiError extends Error {
  readonly problem: Problem;
  readonly status: number;
  constructor(problem: Problem) {
    super(problem.detail ?? problem.title);
    this.name = "ApiError";
    this.problem = problem;
    this.status = problem.status;
  }

  /** Field-level validation failures, if any. */
  get fieldErrors(): FieldError[] {
    return this.problem.errors ?? [];
  }

  /** True for 401 responses (drives token-refresh / re-login logic). */
  get isUnauthorized(): boolean {
    return this.status === 401;
  }
}

/** Best-effort coercion of an arbitrary body into a {@link Problem}. */
export function toProblem(body: unknown, fallbackStatus: number): Problem {
  if (body && typeof body === "object" && "status" in body) {
    const p = body as Partial<Problem>;
    return {
      type: p.type ?? "about:blank",
      title: p.title ?? "Error",
      status: typeof p.status === "number" ? p.status : fallbackStatus,
      detail: p.detail,
      instance: p.instance,
      errors: p.errors,
    };
  }
  return {
    type: "about:blank",
    title: "Error",
    status: fallbackStatus,
    detail: typeof body === "string" && body ? body : undefined,
  };
}

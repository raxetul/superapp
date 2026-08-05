/**
 * Shared, typed API models mirroring the backend contract
 * (`backend/core/src/response.rs`).
 *
 * - Successful (2xx) responses use the house success envelope, served as
 *   `application/json`.
 * - Error (non-2xx) responses use RFC 9457 Problem Details, served as
 *   `application/problem+json`.
 */

/** Pagination metadata for collection responses. */
export interface Pagination {
  page: number;
  per_page: number;
  total_items: number;
  total_pages: number;
}

/** The house success envelope wrapping every 2xx payload. */
export interface SuccessEnvelope<T> {
  success: true;
  data: T;
  message?: string;
  pagination?: Pagination;
}

/** A single field-level validation failure (RFC 9457 extension member). */
export interface FieldError {
  /** JSON Pointer (RFC 6901) into the request body, e.g. `/email`. */
  pointer: string;
  detail: string;
}

/** An RFC 9457 Problem Details document. */
export interface ProblemDetails {
  type: string;
  title: string;
  status: number;
  detail?: string;
  instance?: string;
  errors?: FieldError[];
  /** Arbitrary extension members are flattened onto the document. */
  [key: string]: unknown;
}

/** Type guard: does `value` look like an RFC 9457 Problem Details body? */
export function isProblemDetails(value: unknown): value is ProblemDetails {
  if (typeof value !== 'object' || value === null) return false;
  const v = value as Record<string, unknown>;
  return typeof v.title === 'string' && typeof v.status === 'number';
}

/** Type guard: does `value` look like the house success envelope? */
export function isSuccessEnvelope(value: unknown): value is SuccessEnvelope<unknown> {
  if (typeof value !== 'object' || value === null) return false;
  const v = value as Record<string, unknown>;
  return v.success === true && 'data' in v;
}

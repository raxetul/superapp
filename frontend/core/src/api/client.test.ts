import { describe, it, expect, vi } from "vitest";
import { ApiClient, type FetchLike } from "./client";
import { ApiError } from "./problem";
import { createApi } from "./endpoints";
import type { Capabilities, CurrentUser } from "./types";

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

function problemResponse(problem: unknown, status: number): Response {
  return new Response(JSON.stringify(problem), {
    status,
    headers: { "content-type": "application/problem+json" },
  });
}

describe("TR-07-002 typed API client", () => {
  it("unwraps the house success envelope and returns typed data", async () => {
    const fetchImpl: FetchLike = vi.fn(async () =>
      jsonResponse({
        success: true,
        data: { self_registration_enabled: true, oidc_configured: true },
      }),
    );
    const client = new ApiClient({ baseUrl: "http://api", fetchImpl });
    const caps = await client.get<Capabilities>("/api/v1/auth/capabilities");
    expect(caps.self_registration_enabled).toBe(true);
    expect(caps.oidc_configured).toBe(true);
  });

  it("injects the Authorization header when a token is present", async () => {
    const fetchImpl = vi.fn<FetchLike>(async () =>
      jsonResponse({ success: true, data: { ok: 1 } }),
    );
    const client = new ApiClient({
      baseUrl: "http://api",
      fetchImpl,
      getToken: () => "tok-123",
    });
    await client.get("/api/v1/auth/me");
    const init = fetchImpl.mock.calls[0][1] as RequestInit;
    expect((init.headers as Record<string, string>)["Authorization"]).toBe(
      "Bearer tok-123",
    );
  });

  it("omits Authorization when there is no token", async () => {
    const fetchImpl = vi.fn<FetchLike>(async () =>
      jsonResponse({ success: true, data: {} }),
    );
    const client = new ApiClient({ baseUrl: "http://api", fetchImpl });
    await client.get("/x");
    const init = fetchImpl.mock.calls[0][1] as RequestInit;
    expect(
      (init.headers as Record<string, string>)["Authorization"],
    ).toBeUndefined();
  });

  it("surfaces an RFC 9457 problem as a typed ApiError", async () => {
    const fetchImpl: FetchLike = vi.fn(async () =>
      problemResponse(
        {
          type: "https://superapp/errors/validation",
          title: "Unprocessable Entity",
          status: 422,
          detail: "invalid body",
          errors: [{ pointer: "/email", detail: "must be a valid email" }],
        },
        422,
      ),
    );
    const client = new ApiClient({ baseUrl: "http://api", fetchImpl });
    await expect(client.post("/api/v1/admin/allowlist", {})).rejects.toThrow(
      ApiError,
    );
    try {
      await client.post("/api/v1/admin/allowlist", {});
    } catch (e) {
      const err = e as ApiError;
      expect(err.status).toBe(422);
      expect(err.problem.type).toBe("https://superapp/errors/validation");
      expect(err.fieldErrors[0]).toEqual({
        pointer: "/email",
        detail: "must be a valid email",
      });
    }
  });

  it("treats a problem+json body as an error even on a 2xx-shaped status", async () => {
    const fetchImpl: FetchLike = vi.fn(async () =>
      problemResponse({ title: "Bad", status: 400 }, 400),
    );
    const client = new ApiClient({ baseUrl: "http://api", fetchImpl });
    await expect(client.get("/x")).rejects.toBeInstanceOf(ApiError);
  });

  it("refreshes once and retries on 401 when a handler is provided", async () => {
    let call = 0;
    const fetchImpl: FetchLike = vi.fn(async () => {
      call += 1;
      if (call === 1) return problemResponse({ title: "nope", status: 401 }, 401);
      return jsonResponse({ success: true, data: { pid: "p1" } });
    });
    const onUnauthorized = vi.fn(async () => true);
    const client = new ApiClient({
      baseUrl: "http://api",
      fetchImpl,
      onUnauthorized,
    });
    const me = await client.get<CurrentUser>("/api/v1/auth/me");
    expect(onUnauthorized).toHaveBeenCalledOnce();
    expect(call).toBe(2);
    expect(me.pid).toBe("p1");
  });

  it("does not retry when the refresh handler declines", async () => {
    const fetchImpl: FetchLike = vi.fn(async () =>
      problemResponse({ title: "nope", status: 401 }, 401),
    );
    const onUnauthorized = vi.fn(async () => false);
    const client = new ApiClient({
      baseUrl: "http://api",
      fetchImpl,
      onUnauthorized,
    });
    await expect(client.get("/api/v1/auth/me")).rejects.toBeInstanceOf(ApiError);
    expect(onUnauthorized).toHaveBeenCalledOnce();
    expect(fetchImpl).toHaveBeenCalledOnce();
  });

  it("builds correct URLs and methods through the typed facade", async () => {
    const fetchImpl = vi.fn<FetchLike>(async () =>
      jsonResponse({ success: true, data: { email: "a@b.c", role: "user" } }),
    );
    const client = new ApiClient({ baseUrl: "http://api/", fetchImpl });
    const api = createApi(client);
    await api.setUserRole("a@b.c", "admin");
    const [url, init] = fetchImpl.mock.calls[0];
    expect(url).toBe("http://api/api/v1/admin/users/role");
    expect((init as RequestInit).method).toBe("PUT");
    expect(JSON.parse((init as RequestInit).body as string)).toEqual({
      email: "a@b.c",
      role: "admin",
    });
  });
});

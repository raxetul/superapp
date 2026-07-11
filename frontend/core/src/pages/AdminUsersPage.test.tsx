import { describe, it, expect, vi } from "vitest";
import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AdminUsersPage } from "./AdminUsersPage";
import { renderWithProviders, stubApi } from "@/test/renderApp";
import type { AllowlistEntry } from "@/api/types";
import { ApiError } from "@/api/problem";

describe("FR-07-003 admin user-management UI", () => {
  it("lists allow-listed users returned by the API", async () => {
    const entries: AllowlistEntry[] = [
      { email: "a@buyutech.com.tr", role: "admin" },
      { email: "b@buyutech.com.tr", role: "user" },
    ];
    renderWithProviders(<AdminUsersPage />, {
      authenticated: true,
      api: stubApi({ listAllowlist: async () => entries }),
    });
    expect(await screen.findByText("a@buyutech.com.tr")).toBeInTheDocument();
    expect(screen.getByText("b@buyutech.com.tr")).toBeInTheDocument();
  });

  it("allow-lists a new user by email", async () => {
    const addToAllowlist = vi.fn(async (email: string) => ({
      email,
      role: "user" as const,
    }));
    renderWithProviders(<AdminUsersPage />, {
      authenticated: true,
      api: stubApi({ addToAllowlist, listAllowlist: async () => [] }),
    });
    await screen.findByText("No allow-listed users yet.");
    await userEvent.type(
      screen.getByLabelText("Email"),
      "new@buyutech.com.tr",
    );
    await userEvent.click(
      screen.getByRole("button", { name: /Add to allow-list/ }),
    );
    await waitFor(() =>
      expect(addToAllowlist).toHaveBeenCalledWith("new@buyutech.com.tr"),
    );
  });

  it("changes a user's role", async () => {
    const setUserRole = vi.fn(async (email: string, role: "admin" | "user") => ({
      email,
      role,
    }));
    renderWithProviders(<AdminUsersPage />, {
      authenticated: true,
      api: stubApi({
        listAllowlist: async () => [{ email: "u@x.io", role: "user" }],
        setUserRole,
      }),
    });
    await userEvent.click(await screen.findByRole("button", { name: /Make admin/ }));
    await waitFor(() =>
      expect(setUserRole).toHaveBeenCalledWith("u@x.io", "admin"),
    );
  });

  it("surfaces an RFC 9457 field error from the API", async () => {
    const addToAllowlist = vi.fn(async () => {
      throw new ApiError({
        type: "https://superapp/errors/validation",
        title: "Unprocessable Entity",
        status: 422,
        errors: [{ pointer: "/email", detail: "must be a valid email" }],
      });
    });
    renderWithProviders(<AdminUsersPage />, {
      authenticated: true,
      api: stubApi({ addToAllowlist, listAllowlist: async () => [] }),
    });
    await screen.findByText("No allow-listed users yet.");
    await userEvent.type(screen.getByLabelText("Email"), "bad@x.io");
    await userEvent.click(
      screen.getByRole("button", { name: /Add to allow-list/ }),
    );
    expect(
      await screen.findByText("/email: must be a valid email"),
    ).toBeInTheDocument();
  });
});

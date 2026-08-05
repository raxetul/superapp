import { describe, it, expect } from "vitest";
import { screen } from "@testing-library/react";
import { AppRoutes } from "./AppRoutes";
import {
  renderWithProviders,
  stubApi,
} from "@/test/renderApp";
import type { CurrentUser } from "@/api/types";

const adminUser: CurrentUser = {
  pid: "a1",
  email: "admin@buyutech.com.tr",
  name: "Admin",
  role: "admin",
};

describe("TR-07-004 role-based route protection", () => {
  it("redirects an unauthenticated visitor to the login screen", async () => {
    renderWithProviders(<AppRoutes />, {
      authenticated: false,
      initialEntries: ["/"],
    });
    expect(await screen.findByText("Sign in to SuperApp")).toBeInTheDocument();
  });

  it("renders the dashboard for an authenticated user", async () => {
    renderWithProviders(<AppRoutes />, {
      authenticated: true,
      initialEntries: ["/"],
    });
    expect(await screen.findByText(/Welcome, Test User/)).toBeInTheDocument();
  });

  it("blocks a non-admin from an admin route (redirect to /forbidden)", async () => {
    renderWithProviders(<AppRoutes />, {
      authenticated: true,
      initialEntries: ["/admin/users"],
    });
    expect(await screen.findByText(/403 — Forbidden/)).toBeInTheDocument();
  });

  it("allows an admin to reach the admin route", async () => {
    renderWithProviders(<AppRoutes />, {
      authenticated: true,
      initialEntries: ["/admin/users"],
      api: stubApi({ getMe: async () => adminUser }),
    });
    expect(
      await screen.findByRole("button", { name: /Add to allow-list/ }),
    ).toBeInTheDocument();
  });
});

describe("FR-07-002 role-based UI adaptation", () => {
  it("hides admin navigation from ordinary users", async () => {
    renderWithProviders(<AppRoutes />, {
      authenticated: true,
      initialEntries: ["/"],
    });
    await screen.findByText(/Welcome/);
    expect(screen.queryByTestId("nav-admin-users")).not.toBeInTheDocument();
    expect(screen.queryByTestId("nav-admin-modules")).not.toBeInTheDocument();
  });

  it("shows admin navigation to admins", async () => {
    renderWithProviders(<AppRoutes />, {
      authenticated: true,
      initialEntries: ["/"],
      api: stubApi({ getMe: async () => adminUser }),
    });
    await screen.findByText(/Welcome/);
    expect(screen.getByTestId("nav-admin-users")).toBeInTheDocument();
    expect(screen.getByTestId("nav-admin-modules")).toBeInTheDocument();
  });
});

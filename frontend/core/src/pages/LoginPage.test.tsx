import { describe, it, expect, vi } from "vitest";
import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { LoginPage } from "./LoginPage";
import { renderWithProviders, stubApi } from "@/test/renderApp";

describe("FR-07-001 login via Rauthy", () => {
  it("starts the OIDC redirect to Rauthy when signing in", async () => {
    const redirect = vi.fn();
    renderWithProviders(<LoginPage />, {
      authenticated: false,
      redirect,
    });
    await userEvent.click(
      await screen.findByRole("button", { name: /Sign in with Rauthy/ }),
    );
    await waitFor(() => expect(redirect).toHaveBeenCalledOnce());
    const url = new URL(redirect.mock.calls[0][0]);
    expect(url.pathname).toBe("/auth/v1/oidc/authorize");
    expect(url.searchParams.get("code_challenge_method")).toBe("S256");
    expect(url.searchParams.get("client_id")).toBe("superapp-web");
  });
});

describe("FR-07-004 conditional self-registration UI", () => {
  it("hides the register action when self-registration is disabled", async () => {
    renderWithProviders(<LoginPage />, {
      authenticated: false,
      api: stubApi({
        getCapabilities: async () => ({
          self_registration_enabled: false,
          oidc_configured: true,
        }),
      }),
    });
    await screen.findByRole("button", { name: /Sign in with Rauthy/ });
    expect(screen.queryByTestId("register-button")).not.toBeInTheDocument();
  });

  it("shows the register action only when self-registration is enabled", async () => {
    renderWithProviders(<LoginPage />, {
      authenticated: false,
      api: stubApi({
        getCapabilities: async () => ({
          self_registration_enabled: true,
          oidc_configured: true,
        }),
      }),
    });
    expect(await screen.findByTestId("register-button")).toBeInTheDocument();
  });
});

/**
 * TR-07-003 — OIDC redirect callback. Exchanges the authorization code for
 * tokens, then navigates to the originally requested route.
 */
import * as React from "react";
import { useNavigate } from "react-router-dom";
import { useAuth } from "@/auth/AuthContext";

export function CallbackPage(): React.JSX.Element {
  const { completeLogin } = useAuth();
  const navigate = useNavigate();
  const [error, setError] = React.useState<string | null>(null);

  React.useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const returnTo = await completeLogin(window.location.search);
        if (!cancelled) navigate(returnTo, { replace: true });
      } catch (e) {
        if (!cancelled) setError((e as Error).message);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [completeLogin, navigate]);

  return (
    <main className="flex min-h-screen items-center justify-center p-6">
      {error ? (
        <p role="alert" className="text-destructive">
          Sign-in failed: {error}
        </p>
      ) : (
        <p role="status">Completing sign-in…</p>
      )}
    </main>
  );
}

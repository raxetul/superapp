/**
 * FR-07-001 — Login via Rauthy. FR-07-004 — conditional self-registration.
 *
 * A single "Sign in" action starts the OIDC flow (Rauthy offers SSO and
 * username/password). A "Create account" action is rendered *only* when the
 * backend reports `self_registration_enabled`.
 */
import { useLocation } from "react-router-dom";
import { useAuth } from "@/auth/AuthContext";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";

interface LocationState {
  from?: string;
}

export function LoginPage(): React.JSX.Element {
  const { login, capabilities } = useAuth();
  const location = useLocation();
  const from = (location.state as LocationState | null)?.from ?? "/";

  return (
    <main className="mx-auto flex min-h-screen max-w-md flex-col justify-center p-6">
      <Card>
        <CardHeader>
          <CardTitle>Sign in to SuperApp</CardTitle>
          <CardDescription>
            Authenticate with your organization account.
          </CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-3">
          <Button onClick={() => login(from)}>Sign in with Rauthy</Button>
        </CardContent>
        {capabilities?.self_registration_enabled ? (
          <CardFooter className="flex flex-col items-start gap-2">
            <CardDescription>Don&apos;t have an account?</CardDescription>
            <Button
              variant="outline"
              onClick={() => login(from)}
              data-testid="register-button"
            >
              Create account
            </Button>
          </CardFooter>
        ) : null}
      </Card>
    </main>
  );
}

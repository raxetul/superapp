/**
 * Application composition root: resolves config (TR-07-007), mounts the auth
 * session (TR-07-003) and the router (TR-07-004) inside a browser router.
 *
 * The frontend module host (TR-07-005) is created here; core ships with no
 * bundled modules, so its route/nav sets start empty and are populated as
 * modules register.
 */
import { BrowserRouter } from "react-router-dom";
import { getConfig } from "@/config/env";
import { AuthProvider } from "@/auth/AuthContext";
import { AppRoutes } from "@/routes/AppRoutes";

export function App(): React.JSX.Element {
  const config = getConfig();
  return (
    <BrowserRouter>
      <AuthProvider config={config}>
        <AppRoutes />
      </AuthProvider>
    </BrowserRouter>
  );
}

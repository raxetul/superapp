/**
 * TR-07-004 — Role-based route protection (React Router v6).
 *
 * Unauthenticated users are redirected to `/login` (preserving the attempted
 * location); an authenticated user hitting a route above their role is sent to
 * `/forbidden`. Authorization remains server-side — this only reflects it.
 */
import * as React from "react";
import { Navigate, Outlet, useLocation } from "react-router-dom";
import { useAuth } from "@/auth/AuthContext";
import type { Role } from "@/api/types";

export interface ProtectedRouteProps {
  requireRole?: Role;
  children?: React.ReactNode;
}

export function ProtectedRoute({
  requireRole,
  children,
}: ProtectedRouteProps): React.JSX.Element {
  const { status, user, isAdmin } = useAuth();
  const location = useLocation();

  if (status === "loading") {
    return (
      <div role="status" aria-live="polite" className="p-8 text-muted-foreground">
        Loading…
      </div>
    );
  }

  if (status === "unauthenticated" || !user) {
    return <Navigate to="/login" replace state={{ from: location.pathname }} />;
  }

  if (requireRole === "admin" && !isAdmin) {
    return <Navigate to="/forbidden" replace />;
  }

  return <>{children ?? <Outlet />}</>;
}

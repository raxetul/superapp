/**
 * TR-07-004 / TR-07-005 — Application route table.
 *
 * Public routes (`/login`, `/auth/callback`, `/forbidden`) sit outside the
 * protected shell. Everything under the shell requires authentication; admin
 * routes additionally require the `admin` role. Module-contributed routes
 * (already permission-filtered by the host) are mounted inside the shell.
 */
import * as React from "react";
import { Route, Routes } from "react-router-dom";
import { AppLayout } from "@/components/layout/AppLayout";
import { ProtectedRoute } from "./ProtectedRoute";
import { HomePage } from "@/pages/HomePage";
import { LoginPage } from "@/pages/LoginPage";
import { CallbackPage } from "@/pages/CallbackPage";
import { AdminUsersPage } from "@/pages/AdminUsersPage";
import { ModuleAdminPage } from "@/pages/ModuleAdminPage";
import { ForbiddenPage, NotFoundPage } from "@/pages/StatusPages";
import type { FrontendModule, ModuleNavItem, ModuleRoute } from "@/modules/types";

export interface AppRoutesProps {
  /** Permission-filtered module routes (TR-07-005). */
  moduleRoutes?: (ModuleRoute & { moduleId: string })[];
  moduleNav?: ModuleNavItem[];
  modules?: FrontendModule[];
}

export function AppRoutes({
  moduleRoutes = [],
  moduleNav = [],
  modules = [],
}: AppRoutesProps): React.JSX.Element {
  return (
    <Routes>
      <Route path="/login" element={<LoginPage />} />
      <Route path="/auth/callback" element={<CallbackPage />} />
      <Route path="/forbidden" element={<ForbiddenPage />} />

      <Route
        element={
          <ProtectedRoute>
            <AppLayout moduleNav={moduleNav} />
          </ProtectedRoute>
        }
      >
        <Route index element={<HomePage />} />
        <Route
          path="/admin/users"
          element={
            <ProtectedRoute requireRole="admin">
              <AdminUsersPage />
            </ProtectedRoute>
          }
        />
        <Route
          path="/admin/modules"
          element={
            <ProtectedRoute requireRole="admin">
              <ModuleAdminPage modules={modules} />
            </ProtectedRoute>
          }
        />
        {moduleRoutes.map((r) => {
          const Comp = r.component;
          return (
            <Route
              key={`${r.moduleId}:${r.path}`}
              path={r.path}
              element={<Comp />}
            />
          );
        })}
      </Route>

      <Route path="*" element={<NotFoundPage />} />
    </Routes>
  );
}

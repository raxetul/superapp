/**
 * FR-07-002 — Role-based UI adaptation. The shell renders management/admin
 * navigation only for admins; ordinary users never see those entries. Module
 * host nav entries (TR-07-005) the user is permitted to see are appended.
 */
import { Link, NavLink, Outlet } from "react-router-dom";
import { useAuth } from "@/auth/AuthContext";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import type { ModuleNavItem } from "@/modules/types";

export interface AppLayoutProps {
  /** Module-contributed nav already filtered by permission (TR-07-005). */
  moduleNav?: ModuleNavItem[];
}

export function AppLayout({ moduleNav = [] }: AppLayoutProps): React.JSX.Element {
  const { user, isAdmin, logout } = useAuth();

  return (
    <div className="min-h-screen">
      <header className="border-b">
        <div className="container flex flex-wrap items-center gap-4 py-3">
          <Link to="/" className="font-semibold text-primary">
            SuperApp
          </Link>
          <nav className="flex flex-1 items-center gap-1 text-sm">
            <NavItem to="/">Dashboard</NavItem>
            {isAdmin ? (
              <NavItem to="/admin/users" data-testid="nav-admin-users">
                User management
              </NavItem>
            ) : null}
            {isAdmin ? (
              <NavItem to="/admin/modules" data-testid="nav-admin-modules">
                Module admin
              </NavItem>
            ) : null}
            {moduleNav.map((n) => (
              <NavItem key={n.to} to={n.to}>
                {n.label}
              </NavItem>
            ))}
          </nav>
          <div className="flex items-center gap-3 text-sm">
            {user ? (
              <span className="text-muted-foreground">{user.email}</span>
            ) : null}
            <Button size="sm" variant="outline" onClick={() => logout()}>
              Sign out
            </Button>
          </div>
        </div>
      </header>
      <main className="container py-6">
        <Outlet />
      </main>
    </div>
  );
}

function NavItem({
  to,
  children,
  ...rest
}: {
  to: string;
  children: React.ReactNode;
} & Record<string, unknown>) {
  return (
    <NavLink
      to={to}
      end={to === "/"}
      className={({ isActive }) =>
        cn(
          "rounded-md px-3 py-1.5 hover:bg-accent hover:text-accent-foreground",
          isActive && "bg-accent text-accent-foreground",
        )
      }
      {...rest}
    >
      {children}
    </NavLink>
  );
}

/**
 * FR-07-003 — Admin user-management UI: allow-list users by email and manage
 * roles. Reachable only through an admin-guarded route (TR-07-004), so
 * non-admins never render this screen.
 */
import * as React from "react";
import { useAuth } from "@/auth/AuthContext";
import { ApiError } from "@/api/problem";
import type { AllowlistEntry, Role } from "@/api/types";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";

export function AdminUsersPage(): React.JSX.Element {
  const { api } = useAuth();
  const [entries, setEntries] = React.useState<AllowlistEntry[]>([]);
  const [email, setEmail] = React.useState("");
  const [error, setError] = React.useState<string | null>(null);
  const [loading, setLoading] = React.useState(true);

  const refresh = React.useCallback(async () => {
    try {
      setEntries(await api.listAllowlist());
    } catch (e) {
      setError(describe(e));
    } finally {
      setLoading(false);
    }
  }, [api]);

  React.useEffect(() => {
    void refresh();
  }, [refresh]);

  async function onAdd(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    try {
      await api.addToAllowlist(email);
      setEmail("");
      await refresh();
    } catch (err) {
      setError(describe(err));
    }
  }

  async function onSetRole(target: string, role: Role) {
    setError(null);
    try {
      await api.setUserRole(target, role);
      await refresh();
    } catch (err) {
      setError(describe(err));
    }
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>User management</CardTitle>
        <CardDescription>
          Allow-list users by email and manage their roles.
        </CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-6">
        <form onSubmit={onAdd} className="flex flex-col gap-2 sm:flex-row sm:items-end">
          <div className="flex-1">
            <Label htmlFor="email">Email</Label>
            <Input
              id="email"
              type="email"
              required
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              placeholder="person@buyutech.com.tr"
            />
          </div>
          <Button type="submit">Add to allow-list</Button>
        </form>

        {error ? (
          <p role="alert" className="text-sm text-destructive">
            {error}
          </p>
        ) : null}

        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Email</TableHead>
              <TableHead>Role</TableHead>
              <TableHead>Actions</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {loading ? (
              <TableRow>
                <TableCell colSpan={3}>Loading…</TableCell>
              </TableRow>
            ) : entries.length === 0 ? (
              <TableRow>
                <TableCell colSpan={3}>No allow-listed users yet.</TableCell>
              </TableRow>
            ) : (
              entries.map((u) => (
                <TableRow key={u.email}>
                  <TableCell>{u.email}</TableCell>
                  <TableCell>{u.role}</TableCell>
                  <TableCell>
                    <Button
                      size="sm"
                      variant="outline"
                      onClick={() =>
                        onSetRole(u.email, u.role === "admin" ? "user" : "admin")
                      }
                    >
                      Make {u.role === "admin" ? "user" : "admin"}
                    </Button>
                  </TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </CardContent>
    </Card>
  );
}

function describe(e: unknown): string {
  if (e instanceof ApiError) {
    const field = e.fieldErrors[0];
    return field ? `${field.pointer}: ${field.detail}` : e.problem.title;
  }
  return (e as Error).message ?? "Unexpected error";
}

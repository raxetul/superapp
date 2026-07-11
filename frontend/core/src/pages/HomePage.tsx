/**
 * TR-07-001 — baseline authenticated screen built from ShadCN components.
 * FR-07-002 — role-based UI adaptation is handled in the layout nav.
 */
import { useAuth } from "@/auth/AuthContext";
import { Badge } from "@/components/ui/badge";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";

export function HomePage(): React.JSX.Element {
  const { user } = useAuth();
  return (
    <section className="grid gap-4 sm:grid-cols-2">
      <Card>
        <CardHeader>
          <CardTitle>Welcome{user ? `, ${user.name}` : ""}</CardTitle>
          <CardDescription>SuperApp core dashboard</CardDescription>
        </CardHeader>
        <CardContent className="flex items-center gap-2">
          <span className="text-sm text-muted-foreground">Role</span>
          <Badge variant={user?.role === "admin" ? "default" : "secondary"}>
            {user?.role ?? "unknown"}
          </Badge>
        </CardContent>
      </Card>
    </section>
  );
}

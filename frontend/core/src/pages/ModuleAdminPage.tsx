/** Admin-only module administration placeholder (FR-07-002 surface). */
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import type { FrontendModule } from "@/modules/types";

export interface ModuleAdminPageProps {
  modules?: FrontendModule[];
}

export function ModuleAdminPage({
  modules = [],
}: ModuleAdminPageProps): React.JSX.Element {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Module administration</CardTitle>
        <CardDescription>Loaded frontend modules.</CardDescription>
      </CardHeader>
      <CardContent>
        {modules.length === 0 ? (
          <p className="text-sm text-muted-foreground">No modules loaded.</p>
        ) : (
          <ul className="list-disc pl-5 text-sm">
            {modules.map((m) => (
              <li key={m.id}>
                {m.name} <span className="text-muted-foreground">({m.id})</span>
              </li>
            ))}
          </ul>
        )}
      </CardContent>
    </Card>
  );
}

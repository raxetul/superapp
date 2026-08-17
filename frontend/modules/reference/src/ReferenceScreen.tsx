/** The reference module's web screen (TR-09-007). */
import * as React from "react";

export function ReferenceScreen(): React.JSX.Element {
  return React.createElement(
    "div",
    { "data-testid": "reference-screen" },
    "hello from the reference module",
  );
}

import NativeButton from "@vscode-elements/react-elements/dist/components/VscodeButton.js";
import type { ComponentProps } from "react";

/** The custom element's disabled attribute is not native button semantics. */
export default function VscodeButton(
  props: ComponentProps<typeof NativeButton>,
) {
  return (
    <NativeButton
      {...props}
      aria-disabled={props.disabled ? "true" : "false"}
    />
  );
}

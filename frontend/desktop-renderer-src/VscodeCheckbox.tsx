import NativeCheckbox from "@vscode-elements/react-elements/dist/components/VscodeCheckbox.js";
import type { ComponentProps } from "react";

/** The element leaves its shadow input enabled when disabled. */
export default function VscodeCheckbox(
  props: ComponentProps<typeof NativeCheckbox>,
) {
  return (
    <NativeCheckbox
      {...props}
      aria-disabled={props.disabled ? "true" : "false"}
      onClickCapture={(event) => {
        if (props.disabled) event.preventDefault();
        props.onClickCapture?.(event);
      }}
      onKeyDownCapture={(event) => {
        if (props.disabled && (event.key === " " || event.key === "Enter"))
          event.preventDefault();
        props.onKeyDownCapture?.(event);
      }}
      onChange={(event) => {
        if (!props.disabled) props.onChange?.(event);
      }}
    />
  );
}

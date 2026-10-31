import type { ComponentProps } from "react";

/** Native checkbox semantics with the workbench's VS Code theme. */
export default function VscodeCheckbox({
  children,
  ...props
}: Omit<ComponentProps<"input">, "type">) {
  return (
    <label className="workbench-checkbox">
      <input
        {...props}
        type="checkbox"
        aria-disabled={props.disabled ? "true" : "false"}
      />
      <span>{children}</span>
    </label>
  );
}

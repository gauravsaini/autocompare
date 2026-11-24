import type { ReactNode } from "react";
import Select from "./Select";
import type { SelectProps } from "./Select";

/**
 * Wave-158: shared labeled-field primitive (typography ONLY via `ac-meta`,
 * no geometry/color of its own). Replaces ad-hoc
 * `text-[length:var(--ac-font-size-ui)]` label trios (lightningcss rejects
 * them) next to `Input`/`Select` controls. Testids ride the wrapped
 * control, untouched.
 */
export interface FieldProps {
  readonly label: string;
  readonly htmlFor?: string;
  readonly children: ReactNode;
  readonly className?: string;
}

export function Field({ label, htmlFor, children, className = "" }: FieldProps): React.JSX.Element {
  return (
    <label
      htmlFor={htmlFor}
      className={`ac-meta inline-flex shrink-0 items-center gap-[var(--ac-space-1)] font-normal text-[var(--ac-fg-muted)] ${className}`}
    >
      <span>{label}</span>
      {children}
    </label>
  );
}

export default Field;

export interface LabeledSelectProps extends SelectProps {
  readonly label: string;
  readonly htmlFor?: string;
  readonly fieldClassName?: string;
}

/**
 * Wrapper component combining Field and Select.
 */
export function LabeledSelect({
  label,
  htmlFor,
  fieldClassName = "",
  id,
  className = "",
  controlSize,
  children,
  ...rest
}: LabeledSelectProps): React.JSX.Element {
  return (
    <Field label={label} htmlFor={htmlFor ?? id} className={fieldClassName}>
      <Select id={id} className={className} controlSize={controlSize} {...rest}>
        {children}
      </Select>
    </Field>
  );
}

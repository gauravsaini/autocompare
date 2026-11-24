import type { InputHTMLAttributes } from "react";

export interface CheckboxProps extends Omit<InputHTMLAttributes<HTMLInputElement>, "type" | "size"> {
  readonly label: string;
}

/** Crisp 13px macOS-style checkbox with accent fill. Testid rides the input. */
export default function Checkbox({ label, className = "", ...rest }: CheckboxProps): React.JSX.Element {
  return (
    <label className={`inline-flex h-[var(--ac-control-h-sm)] cursor-pointer items-center gap-1.5 ac-chrome-label text-[var(--ac-fg)] ${className}`}>
      <input
        type="checkbox"
        aria-label={label}
        title={label}
        className="h-[var(--ac-checkbox-size)] w-[var(--ac-checkbox-size)] shrink-0 cursor-pointer accent-[var(--ac-accent)]"
        {...rest}
      />
      <span>{label}</span>
    </label>
  );
}

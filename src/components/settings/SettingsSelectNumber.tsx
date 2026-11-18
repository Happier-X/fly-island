import { SelectControl } from "../controls/SelectControl";
import "./SettingsSelect.css";

type NumberOption<T extends number> = { value: T; label: string };

/** The settings dropdown for numeric choices such as an interval in hours. */
export function SettingsSelectNumber<T extends number>({
  value,
  options,
  ariaLabel,
  disabled = false,
  onChange,
}: {
  value: T;
  options: readonly NumberOption<T>[];
  ariaLabel: string;
  disabled?: boolean;
  onChange: (value: T) => void;
}) {
  return (
    <div className="settings-select">
      <SelectControl
        value={String(value)}
        options={options.map((option) => ({ value: String(option.value), label: option.label }))}
        ariaLabel={ariaLabel}
        disabled={disabled}
        onChange={(selected) => onChange(Number(selected) as T)}
      />
    </div>
  );
}

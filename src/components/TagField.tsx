// A single editable tag field.
//
// "Mixed" means the selection doesn't agree on this tag: the box is left empty
// so saving it untouched can't clobber the files that had a different value,
// and the disagreement is shown instead — a badge where there's room, a
// compact "≠" placeholder in the narrow ones.

export interface TagFieldValue {
  value: string;
  mixed: boolean;
}

export interface TagFieldProps extends TagFieldValue {
  label: string;
  onChange: (value: string) => void;
  multiline?: boolean;
  numeric?: boolean;
  /// Half-width variant, laid out two to a row.
  narrow?: boolean;
}

export function TagField({
  label,
  value,
  mixed,
  onChange,
  multiline,
  numeric,
  narrow,
}: TagFieldProps) {
  const placeholder = mixed ? (narrow ? "≠" : "Multiple values") : "";

  return (
    <div className={narrow ? "tag-field tag-field-narrow" : "tag-field"}>
      <div className="tag-field-head">
        <span>{label}</span>
        {mixed && !narrow && <span className="tag-field-badge">multiple values</span>}
      </div>
      {multiline ? (
        <textarea
          rows={2}
          value={value}
          placeholder={placeholder}
          onChange={(ev) => onChange(ev.target.value)}
        />
      ) : (
        <input
          type="text"
          inputMode={numeric ? "numeric" : undefined}
          value={value}
          placeholder={placeholder}
          onChange={(ev) => onChange(ev.target.value)}
        />
      )}
    </div>
  );
}

export interface TagFractionFieldProps {
  label: string;
  /// The numerator (track number, disc number).
  main: TagFieldValue;
  /// The denominator (track total, disc total).
  total: TagFieldValue;
  onMainChange: (value: string) => void;
  onTotalChange: (value: string) => void;
}

/// "3 / 12" style pair — one label over two numeric boxes.
export function TagFractionField({
  label,
  main,
  total,
  onMainChange,
  onTotalChange,
}: TagFractionFieldProps) {
  return (
    <div className="tag-field tag-field-narrow">
      <div className="tag-field-head">
        <span>{label}</span>
      </div>
      <div className="tag-field-pair">
        <input
          type="text"
          inputMode="numeric"
          value={main.value}
          placeholder={main.mixed ? "≠" : ""}
          onChange={(ev) => onMainChange(ev.target.value)}
        />
        <span className="tag-field-sep">/</span>
        <input
          type="text"
          inputMode="numeric"
          value={total.value}
          placeholder={total.mixed ? "≠" : ""}
          onChange={(ev) => onTotalChange(ev.target.value)}
        />
      </div>
    </div>
  );
}

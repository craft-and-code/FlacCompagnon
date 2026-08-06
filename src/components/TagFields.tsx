// The tag panel's editable fields, rendered from `TAG_LAYOUT` rather than
// from hand-written markup — adding a tag is a one-line change there.

import { Search } from "lucide-react";

import type { TagFieldValue } from "./TagField";
import { TagField, TagFractionField } from "./TagField";
import { TAG_LAYOUT, type TagTextField } from "./tagLayout";

export interface TagFieldsProps {
  values: Record<TagTextField, TagFieldValue>;
  onFieldChange: (field: TagTextField, value: string) => void;
  /// `null` = the selection disagrees, shown as an indeterminate checkbox.
  compilation: boolean | null;
  onCompilationChange: (checked: boolean) => void;
  extendedCount: number;
  onOpenExtended: () => void;
  onOpenLookup: () => void;
}

export function TagFields({
  values,
  onFieldChange,
  compilation,
  onCompilationChange,
  extendedCount,
  onOpenExtended,
  onOpenLookup,
}: TagFieldsProps) {
  return (
    <div className="tag-fields">
      {TAG_LAYOUT.map((entry, i) => {
        if (entry.kind === "text") {
          return (
            <TagField
              key={entry.field}
              label={entry.label}
              multiline={entry.multiline}
              {...values[entry.field]}
              onChange={(v) => onFieldChange(entry.field, v)}
            />
          );
        }
        if (entry.kind === "narrow-row") {
          return (
            <div className="tag-field-row" key={`narrow-${i}`}>
              {entry.items.map((item) => (
                <TagField
                  key={item.field}
                  label={item.label}
                  numeric={item.numeric}
                  narrow
                  {...values[item.field]}
                  onChange={(v) => onFieldChange(item.field, v)}
                />
              ))}
            </div>
          );
        }
        return (
          <div className="tag-field-row" key={`fraction-${i}`}>
            {entry.items.map((item) => (
              <TagFractionField
                key={item.field}
                label={item.label}
                main={values[item.field]}
                total={values[item.totalField]}
                onMainChange={(v) => onFieldChange(item.field, v)}
                onTotalChange={(v) => onFieldChange(item.totalField, v)}
              />
            ))}
          </div>
        );
      })}

      <label className="tag-field-checkbox">
        <input
          type="checkbox"
          checked={compilation === true}
          ref={(el) => {
            if (el) el.indeterminate = compilation === null;
          }}
          onChange={(ev) => onCompilationChange(ev.target.checked)}
        />
        <span>Compilation</span>
      </label>

      {extendedCount > 0 && (
        <button className="tag-extended-btn" type="button" onClick={onOpenExtended}>
          Extended tags <span className="tag-extended-count">({extendedCount})</span>
        </button>
      )}

      <button
        className="btn btn-secondary tag-lookup-btn"
        type="button"
        onClick={onOpenLookup}
      >
        <Search className="tag-lookup-icon" size={15} strokeWidth={1.7} />
        Search online…
      </button>
    </div>
  );
}

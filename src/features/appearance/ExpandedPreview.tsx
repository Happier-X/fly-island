import { Bell, Minus, RotateCcw, Settings } from "lucide-react";
import { useTranslation } from "../../i18n";
import "./ExpandedPreview.css";

const PAGE_DOTS = ["todo", "music", "clipboard", "shelf"] as const;
const HEADER_BUTTONS = [Settings, Bell, RotateCcw, Minus];

/**
 * The expanded island in the appearance preview, drawn small: the header (page dots, window
 * buttons) and the todo page, laid out as on the island (IslandHeader, TodoNotebook) with
 * sample todos.
 */
export function ExpandedPreview() {
  const { todos } = useTranslation().settings.appearance.preview;
  const rows = [todos.one, todos.two, todos.three];
  return (
    <div className="expanded-preview" aria-hidden="true">
      <span className="expanded-preview__header">
        <span className="expanded-preview__brand">fly-island</span>
        <span className="expanded-preview__dots">
          {PAGE_DOTS.map((page) => <span key={page} className={`expanded-preview__page expanded-preview__page--${page}`} />)}
        </span>
        <span className="expanded-preview__buttons">
          {HEADER_BUTTONS.map((Icon, index) => (
            <span key={index} className="expanded-preview__button"><Icon size={10} strokeWidth={2} /></span>
          ))}
        </span>
      </span>
      <span className="expanded-preview__list">
        {rows.map((todo, index) => (
          <span key={todo} className={index === 0 ? "expanded-preview__row expanded-preview__row--active" : "expanded-preview__row"}>
            <span className="expanded-preview__check" />
            <span className="expanded-preview__task">{todo}</span>
          </span>
        ))}
      </span>
    </div>
  );
}

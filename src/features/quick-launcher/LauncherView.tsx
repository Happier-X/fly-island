import { useEffect, useMemo, useRef, useState, type CSSProperties, type KeyboardEvent } from "react";
import { AppWindow, File, Globe, Search } from "lucide-react";
import { useTranslation } from "../../i18n";
import { searchLauncher } from "./matching.ts";
import type { LauncherSettings } from "./storage.ts";
import type { LauncherEntry, LauncherResult } from "./types.ts";
import "./LauncherView.css";

/**
 * The launcher panel itself: query field, ranked result list and keyboard control.
 * It has no Tauri calls, so the browser regression page can drive it directly; the
 * window around it supplies entries, icons and the open / close actions.
 */
export function LauncherView({
  entries,
  settings,
  icons,
  loading,
  openToken,
  openError,
  surfaceColor,
  onOpen,
  onClose,
  onResultsChange,
}: {
  entries: readonly LauncherEntry[];
  settings: LauncherSettings;
  icons: Record<string, string>;
  loading: boolean;
  /** Bumped on every open, to clear the query and focus the field. */
  openToken: number;
  openError: string;
  /** The island's background colour, so the panel follows the theme. */
  surfaceColor?: string;
  onOpen: (result: LauncherResult) => void;
  onClose: () => void;
  /** Told the rows on screen, so the window can ask for their icons. */
  onResultsChange?: (results: LauncherResult[]) => void;
}) {
  const t = useTranslation();
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState(-1);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  const results = useMemo(
    () =>
      searchLauncher(entries, query, {
        searchWeb: settings.searchWeb && !loading,
        engine: settings.searchEngine,
      }),
    [entries, query, settings.searchWeb, settings.searchEngine, loading],
  );

  useEffect(() => {
    setQuery("");
    setSelected(-1);
    window.requestAnimationFrame(() => inputRef.current?.focus());
    // A new open always starts from an empty field.
  }, [openToken]);

  useEffect(() => {
    setSelected(results.length > 0 ? 0 : -1);
    onResultsChange?.(results);
  }, [results, onResultsChange]);

  // Keep the highlighted row in view when the arrow keys move past the visible rows.
  useEffect(() => {
    const row = listRef.current?.querySelector<HTMLElement>('[data-selected="true"]');
    row?.scrollIntoView({ block: "nearest" });
  }, [selected]);

  const changeQuery = (next: string) => setQuery(next);

  const openSelected = () => {
    const result = results[selected];
    if (result) onOpen(result);
  };

  const handleKeys = (event: KeyboardEvent<HTMLInputElement>) => {
    // Enter while an input method is composing commits the composition, not the row.
    if (event.nativeEvent.isComposing || event.keyCode === 229) return;
    if (event.key === "ArrowDown") {
      event.preventDefault();
      if (results.length > 0) setSelected((current) => (current + 1) % results.length);
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      if (results.length > 0) setSelected((current) => (current - 1 + results.length) % results.length);
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      openSelected();
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      onClose();
    }
  };

  return (
    <main className="launcher-stage">
      <section
        className="launcher-window"
        role="dialog"
        aria-label={t.launcher.title}
        style={surfaceColor ? ({ "--launcher-surface": surfaceColor } as CSSProperties) : undefined}
      >
        <label className="launcher-search">
          <Search size={18} strokeWidth={1.8} aria-hidden="true" />
          <input
            ref={inputRef}
            className="launcher-search__input"
            type="text"
            value={query}
            spellCheck={false}
            autoComplete="off"
            placeholder={t.launcher.placeholder}
            aria-label={t.launcher.placeholder}
            onChange={(event) => changeQuery(event.currentTarget.value)}
            onKeyDown={handleKeys}
            onBlur={() =>
              window.requestAnimationFrame(() => {
                // Refocus only while the panel still has the focus; a blur that means
                // the whole window is going away must not pull the focus back.
                if (document.hasFocus()) inputRef.current?.focus();
              })
            }
          />
          <kbd className="launcher-search__esc">Esc</kbd>
        </label>

        <div
          ref={listRef}
          className="launcher-results"
          role="listbox"
          aria-label={t.launcher.results}
        >
          {query.trim() === "" ? (
            <p className="launcher-empty">{t.launcher.emptyHint}</p>
          ) : results.length === 0 ? (
            <p className="launcher-empty">{loading ? t.launcher.loading : t.launcher.noResults(query.trim())}</p>
          ) : (
            results.map((result, index) => (
              <button
                key={result.id}
                className={`launcher-result${index === selected ? " launcher-result--selected" : ""}`}
                type="button"
                role="option"
                aria-selected={index === selected}
                data-launcher-id={result.id}
                data-selected={index === selected}
                onMouseDown={(event) => event.preventDefault()}
                onMouseEnter={() => setSelected(index)}
                onClick={() => onOpen(result)}
              >
                <span className="launcher-result__icon" aria-hidden="true">
                  {icons[result.id] ? (
                    <img src={icons[result.id]} alt="" />
                  ) : result.kind === "web" ? (
                    <Globe size={20} strokeWidth={1.6} />
                  ) : result.kind === "file" ? (
                    <File size={20} strokeWidth={1.6} />
                  ) : (
                    <AppWindow size={20} strokeWidth={1.6} />
                  )}
                </span>
                <span className="launcher-result__text">
                  <span className="launcher-result__name">
                    {result.kind === "web" ? t.launcher.searchWeb(result.name) : result.name}
                  </span>
                  <span className="launcher-result__subtitle">{result.subtitle}</span>
                </span>
              </button>
            ))
          )}
        </div>

        <footer className="launcher-footer">
          <span className="launcher-footer__keys">
            <kbd>↑</kbd><kbd>↓</kbd> {t.launcher.hintSelect}
            <kbd>Enter</kbd> {t.launcher.hintOpen}
            <kbd>Esc</kbd> {t.launcher.hintClose}
          </span>
          {openError ? (
            <span className="launcher-footer__error" role="alert">{openError}</span>
          ) : loading ? (
            <span className="launcher-footer__status">{t.launcher.loading}</span>
          ) : null}
        </footer>
      </section>
    </main>
  );
}

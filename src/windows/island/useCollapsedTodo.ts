import { isUpdateTodo } from "../../features/todos/updateTodo";
import { useMemo } from "react";
import type { IslandSettings } from "../../features/appearance/types";
import type { TodoItem } from "../../features/todos/types";
import { useTranslation } from "../../i18n";
import { useTodoRotation } from "./useIslandAutomation";
import { alignCollapsedIslandWidth, getCollapsedIslandWidth } from "./layout";
import { pickCollapsedItem } from "./collapsedItem";

// Lyric lines come and go every few seconds; the island keeps its widest size for them rather
// than resizing with each line.
const LYRICS_WIDTH_TEXT = "♪".repeat(24);
// The song's cover (22px) and the gap after it.
const COVER_WIDTH = 30;

/**
 * What the collapsed island shows: the lyric line while a song plays (`lyricsPlaying`), or for
 * todos the active (or rotating) todo title, otherwise the pending count; whichever comes first in
 * Settings → Appearance → Collapsed island of those that have something to show (`collapsedItem`);
 * a todo only claims its rank once one is started (pickCollapsedItem). Also the width that needs,
 * with room for the song's cover beside the lyric line when `coverWanted`.
 */
export function useCollapsedTodo(
  todos: TodoItem[],
  activeTodoId: string | null,
  settings: IslandSettings,
  isRotationPaused: boolean,
  { lyricsPlaying = false, coverWanted = false }: { lyricsPlaying?: boolean; coverWanted?: boolean } = {},
) {
  const t = useTranslation();
  const pendingTodoIds = useMemo(
    () => todos.filter((todo) => !todo.completed && !isUpdateTodo(todo)).map((todo) => todo.id),
    [todos],
  );
  const displayedTodoId = useTodoRotation(
    pendingTodoIds,
    activeTodoId,
    settings.rotateTodos,
    settings.todoRotationSeconds,
    isRotationPaused,
  );
  const collapsedTaskTitle = useMemo(() => {
    const activeTodo = todos.find(
      (todo) => todo.id === displayedTodoId && !todo.completed,
    );
    return activeTodo?.title ?? null;
  }, [displayedTodoId, todos]);
  // "0 todos left" is not worth a hint; with nothing pending the bar shows what it
  // would without a todo count.
  const pendingTodoCount = pendingTodoIds.length;
  const collapsedTodoCount = settings.showIdleTodoCount && !collapsedTaskTitle && pendingTodoCount > 0
    ? pendingTodoCount
    : null;
  const collapsedItem = pickCollapsedItem(settings.collapsedPriority, {
    lyrics: lyricsPlaying,
    todo: collapsedTaskTitle !== null || collapsedTodoCount !== null,
  }, activeTodoId !== null && pendingTodoIds.includes(activeTodoId));
  const collapsedIslandWidth = useMemo(() => {
    const todoText = collapsedTaskTitle ?? (collapsedTodoCount !== null ? t.island.pendingTodos(collapsedTodoCount) : null);
    const itemText = collapsedItem === "lyrics" ? LYRICS_WIDTH_TEXT : collapsedItem === "todo" ? todoText : null;
    const text = settings.showCollapsedText ? itemText ?? (settings.showTitle ? "fly-island" : "") : "";
    return alignCollapsedIslandWidth(
      getCollapsedIslandWidth(text, coverWanted && collapsedItem === "lyrics" ? COVER_WIDTH : 0),
      window.innerWidth,
      window.devicePixelRatio,
    );
  }, [collapsedItem, collapsedTaskTitle, collapsedTodoCount, coverWanted, settings.showCollapsedText, settings.showTitle, t]);

  return { collapsedTaskTitle, collapsedTodoCount, collapsedItem, collapsedIslandWidth };
}

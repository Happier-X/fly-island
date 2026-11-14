export type TranslationTarget = "window" | "island";

export type AppearanceMode = "classic" | "liquidGlass";
export type IslandFont = "system" | "pixel";
/** Shape of the island's corners: CSS `corner-shape: squircle`, or a circular arc. */
export type IslandCornerShape = "squircle" | "round";
/** Which monitor the island sits on: the primary one, or it follows the pointer or the foreground window there. */
export type IslandMonitor = "primary" | "cursor" | "foreground";
/** How the collapsed island opens: a click, or resting the pointer on it. */
export type ExpandTrigger = "click" | "hover";
/** What the expanded island's minimize button does: hide to the tray, or tuck to the top edge. */
export type MinimizeAction = "tray" | "tuck";
/** What resting the pointer on an island tucked to the edge does: slide it out, or open its panel too. */
export type TuckedHoverAction = "reveal" | "expand";
/** What double-clicking a file shelf entry does. */
export type ShelfDoubleClick = "open" | "reveal";
/** What the collapsed island can show in its text: the lyric line, or a todo. */
export type CollapsedItem = "lyrics" | "todo";
/**
 * What sits on the collapsed island, in the order the user arranged it (Appearance → Collapsed
 * island): the text (pulse, cover and the lyric or todo line) and the status icons —
 * the new-mail badge, files on the shelf, the music waveform, Cindy notifications (or the tuck
 * dot). Icons before the text sit on its left.
 */
export type CollapsedSlot = "text" | "mail" | "shelf" | "music" | "notifications";
/** The page dots in the expanded island's header, which the user can drag into any order. */
export type PageDot = "todo" | "music" | "clipboard" | "shelf" | "translation";

export type IslandSettings = {
  topEdgeLayoutVersion: 1;
  appearanceMode: AppearanceMode;
  fontFamily: IslandFont;
  glassIntensity: number;
  opacity: number;
  collapsedScale: number;
  expandedScale: number;
  marginY: number;
  /** Prevent moving the island by dragging; explicit position reset still works. */
  positionLocked: boolean;
  islandMonitor: IslandMonitor;
  /** Bottom corner radius (px) of the expanded island; the collapsed, detached and inner panel radii follow it. */
  islandCornerRadius: number;
  islandCornerShape: IslandCornerShape;
  /** Tallest expanded panel (px) before the todo list, clipboard and messages scroll. */
  maxPanelHeight: number;
  /** Clipboard history rows the expanded clipboard page shows before it scrolls. */
  clipboardVisibleRows: number;
  taskTextColor: string;
  pulseColor: string;
  pulseBrightness: number;
  islandBackgroundColor: string;
  todoBackgroundColor: string;
  showTitle: boolean;
  showCollapsedText: boolean;
  showIdleTodoCount: boolean;
  carryOverIncompleteTodos: boolean;
  enableTodoReorder: boolean;
  rotateTodos: boolean;
  todoRotationSeconds: number;
  autoTuck: boolean;
  autoTuckSeconds: number;
  /** Seconds without use before an open panel closes by itself; 0 never (AUTO_COLLAPSE_SECONDS). */
  autoCollapseSeconds: number;
  /** An island tucked after idling slides back once the cursor that brought it out leaves. */
  autoTuckRetract: boolean;
  /** How long (ms) it waits after the cursor leaves before sliding back. */
  autoTuckRetractDelayMs: number;
  /** A tucked island slides back into view to show a new message preview. */
  revealOnMessage: boolean;
  expandTrigger: ExpandTrigger;
  /** Pointer rest time (ms) before a hover opens the island. */
  hoverExpandDelayMs: number;
  /** Time (ms) a hover-opened panel stays open after the pointer leaves it. */
  hoverCollapseDelayMs: number;
  minimizeAction: MinimizeAction;
  tuckedHoverAction: TuckedHoverAction;
  /**
   * Module switches. Off hides the module on the island and stops its background work;
   * its data is kept. QQ, WeChat and clipboard history use their existing switches.
   */
  todoEnabled: boolean;
  musicEnabled: boolean;
  /** Cindy notifications: the collapsed island's count and all background reads stop when off. */
  cindyNotificationsEnabled: boolean;
  cindyNotifyDone: boolean;
  cindyNotifyQuestion: boolean;
  cindyNotifyScheduled: boolean;
  cindyNotifyError: boolean;
  shelfEnabled: boolean;
  /** Off hides the island's translate button and releases the global translation shortcut. */
  translationEnabled: boolean;
  /** Where translation opens: its own window, or a page of the expanded island. */
  translationTarget: TranslationTarget;
  /** Dragging files onto a collapsed or tucked island opens the shelf as a drop zone. */
  shelfExpandOnDrop: boolean;
  /** Days after which shelf entries leave the list (0 keeps them). */
  shelfAutoCleanupDays: number;
  /** Entries whose file was moved or deleted leave the list instead of showing as missing. */
  shelfRemoveMissing: boolean;
  /** An entry leaves the list once it has been dragged out somewhere. */
  shelfRemoveAfterDragOut: boolean;
  shelfMaxItems: number;
  shelfDoubleClick: ShelfDoubleClick;
  /** The collapsed island shows how many files are on the shelf. */
  shelfShowCount: boolean;
  /** Folder of .lrc files searched for the playing song's lyrics; empty for none. */
  lyricsFolder: string;
  /** Lyrics not in the folder are looked up on LRCLIB. */
  lyricsOnline: boolean;
  /** With online lyrics, also ask Kugou and NetEase (unofficial sources). */
  lyricsChinese: boolean;
  /** Lyrics shown this many ms later (positive) or earlier than the position Windows reports. */
  lyricsOffsetMs: number;
  mailEnabled: boolean;
  /** Which of the lyric line and a todo the collapsed island shows first when both have something. */
  collapsedPriority: CollapsedItem[];
  /** The playing song's cover on the collapsed island. */
  collapsedShowCover: boolean;
  /** Where the text and each status icon sit on the collapsed island, left to right. */
  collapsedLayout: CollapsedSlot[];
  /** Order of the page dots in the expanded island's header. */
  pageOrder: PageDot[];
  /** While a full-screen app (game, video, slides) is in front: no hover opening or focus, no message pop-ups. */
  gameDoNotDisturb: boolean;
  /** While a full-screen app is in front, the island tucks to the top edge and comes back after. */
  gameAutoTuck: boolean;
};

export type LegacyIslandSettings = Partial<IslandSettings> & {
  margin?: number;
  sizeScale?: number;
  taskTitleColor?: string;
  pendingTodoColor?: string;
};

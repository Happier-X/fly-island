import { getEffectiveAppearanceMode } from "../../features/appearance/settings";
import { useState, type ReactNode } from "react";
import type { AppearanceMode, CollapsedItem, CollapsedSlot, PageDot } from "../../features/appearance/types";
import type { MediaState } from "../../features/media/api";
import { latestMessagePreview, type MessageSource } from "../../features/messages/messagePreview";
import type { MailController } from "../../features/messages/mail/MailPanel";
import type { QQNotificationsController } from "../../features/messages/qq-notifications/QQNotifications";
import type { WeChatRemindersController } from "../../features/messages/wechat/WeChatReminders";
import { useTranslation } from "../../i18n";
import type { IslandFeatures } from "./features";
import { IslandCollapsedBar } from "./IslandCollapsedBar";
import { IslandHeader } from "./IslandHeader";
import type { IslandMode, IslandPage, NativeGlassState } from "./types";
import { useGlassPointer } from "./useGlassPointer";
import { useIslandPreparing } from "./useIslandPreparing";
import { useMotionLayer } from "./useMotionLayer";
import { usePageFadeIn } from "./usePageFadeIn";
import type { LyricsSettings } from "../../features/media/lyrics/useLyrics";
import "./IslandShell.css";

type IslandShellProps = {
  qqMail: MailController;
  qqNotifications: QQNotificationsController;
  wechatReminders: WeChatRemindersController;
  onOpenMessages: (source: MessageSource) => void;
  mode: IslandMode;
  page: IslandPage;
  /** The open page's height (unscaled); a change resizes the shape around the panel. */
  expandedHeight: number;
  appearanceMode: AppearanceMode;
  nativeGlassState: NativeGlassState;
  isTucked: boolean;
  positionLocked: boolean;
  isDetached: boolean;
  showTitle: boolean;
  showCollapsedText: boolean;
  collapsedTaskTitle: string | null;
  collapsedTodoCount: number | null;
  /** Which of the lyric line and a todo the collapsed island's text shows, if any. */
  collapsedItem: CollapsedItem | null;
  collapsedLayout: readonly CollapsedSlot[];
  /** The playing song's cover for the collapsed island, or null when it is not shown. */
  collapsedCover: string | null;
  /** Where the collapsed island's lyric line comes from. */
  lyricsSettings: LyricsSettings & { lyricsOffsetMs: number };
  /** Order of the header's page dots, and saving a new one after a drag. */
  pageOrder: PageDot[];
  onPageOrderChange: (order: PageDot[]) => void;
  /** Cindy notifications waiting, shown in place of the tuck dot; null while that module is off. */
  notificationCount: number | null;
  /** What the notification badge reads out; null when there is none. */
  notificationLabel: string | null;
  mediaState: MediaState;
  /** Modules switched on; the header and collapsed bar hide the rest. */
  features: IslandFeatures;
  /** Files on the shelf for the collapsed bar, or null when it does not show them. */
  shelfCount: number | null;
  onOpenPage: (page: IslandPage) => void;
  onOpenNotifications: () => void;
  onWindowDragStart: () => void;
  onCollapse: () => void;
  onResetPosition: () => void;
  minimizesToTray: boolean;
  onMinimize: () => void;
  /** Null while translation is switched off, which hides its button. */
  onOpenTranslator: (() => void) | null;
  onOpenSettings: () => void;
  onTuck: () => void;
  onReveal: () => void;
  onPageChange: (page: IslandPage) => void;
  children: ReactNode;
};

export function IslandShell({
  qqMail,
  qqNotifications,
  wechatReminders,
  onOpenMessages,
  mode,
  page,
  expandedHeight,
  appearanceMode,
  nativeGlassState,
  isTucked,
  positionLocked,
  isDetached,
  showTitle,
  showCollapsedText,
  collapsedTaskTitle,
  collapsedTodoCount,
  collapsedItem,
  collapsedLayout,
  collapsedCover,
  lyricsSettings,
  pageOrder,
  onPageOrderChange,
  notificationCount,
  notificationLabel,
  mediaState,
  features,
  shelfCount,
  onOpenPage,
  onOpenNotifications,
  onWindowDragStart,
  onCollapse,
  onResetPosition,
  minimizesToTray,
  onMinimize,
  onOpenTranslator,
  onOpenSettings,
  onTuck,
  onReveal,
  onPageChange,
  children,
}: IslandShellProps) {
  const t = useTranslation();
  const isExpanded = mode === "expanded";
  const effectiveAppearanceMode = getEffectiveAppearanceMode(appearanceMode);
  const isLiquidGlass = effectiveAppearanceMode === "liquidGlass";
  const { handleGlassPointerMove, resetGlassPointer } = useGlassPointer(isLiquidGlass);
  const isPreparing = useIslandPreparing(isExpanded);
  const contentRef = usePageFadeIn<HTMLDivElement>(page, mode);
  const panelRef = useMotionLayer<HTMLDivElement>(mode, page, expandedHeight);
  const qqPreview = qqNotifications.preview;
  const qqNotificationCount = qqNotifications.snapshot.items.length;
  const wechatPending = wechatReminders.snapshot.pending;
  const wechatPreviewAt = wechatReminders.previewReceivedAt;
  const [latestPreviewAt, setLatestPreviewAt] = useState(0);
  const preview = latestMessagePreview([
    qqPreview ? { ...qqPreview, source: "qq", receivedAt: qqNotifications.previewReceivedAt ?? 0 } : null,
    wechatPreviewAt !== null ? { source: "wechat", title: t.messages.wechat, body: t.messages.newReminder, receivedAt: wechatPreviewAt } : null,
    qqMail.preview ? { source: "qqMail", title: qqMail.preview.sender, body: qqMail.preview.subject, receivedAt: qqMail.previewReceivedAt ?? 0 } : null,
  ], latestPreviewAt);
  if (preview && preview.receivedAt > latestPreviewAt) setLatestPreviewAt(preview.receivedAt);
  const className = [
    "island",
    `island--${mode}`,
    `island--${page}`,
    `island--appearance-${effectiveAppearanceMode}`,
    isPreparing && !isExpanded ? "island--preparing" : "",
    isDetached ? "island--detached" : "island--docked",
    isTucked ? "island--tucked" : "",
    isLiquidGlass ? `island--glass-${nativeGlassState}` : "",
    showTitle ? "" : "island--title-hidden",
  ]
    .filter(Boolean)
    .join(" ");
  const collapsedLabel = showCollapsedText
    ? collapsedTaskTitle
      ? t.messages.collapsedTask(collapsedTaskTitle)
      : collapsedTodoCount !== null
        ? t.island.pendingTodos(collapsedTodoCount)
        : "fly-island"
    : "fly-island";

  return (
    <section
      className={className}
      aria-label={collapsedLabel}
      onClick={(event) => {
        if (!isExpanded && event.target === event.currentTarget) {
          onOpenPage(page);
        }
      }}
      onPointerMove={handleGlassPointerMove}
      onPointerLeave={resetGlassPointer}
      onMouseEnter={() => {
        if (isTucked) {
          onReveal();
        }
      }}
    >
      <div className="island__surface">
        {isLiquidGlass && (
          <div className="island__glass" aria-hidden="true">
            <span className="island__glass-refraction" />
            <span className="island__glass-highlight" />
          </div>
        )}
        <IslandCollapsedBar
          qqMail={qqMail}
          qqNotifications={qqNotifications}
          wechatReminders={wechatReminders}
          preview={preview}
          page={page}
          isExpanded={isExpanded}
          isTucked={isTucked}
          positionLocked={positionLocked}
          showTitle={showTitle}
          showCollapsedText={showCollapsedText}
          collapsedLabel={collapsedLabel}
          collapsedTaskTitle={collapsedTaskTitle}
          collapsedTodoCount={collapsedTodoCount}
          collapsedItem={collapsedItem}
          collapsedLayout={collapsedLayout}
          collapsedCover={collapsedCover}
          lyricsSettings={lyricsSettings}
          notificationCount={notificationCount}
          notificationLabel={notificationLabel}
          mediaState={mediaState}
          showMusic={features.music}
          shelfCount={shelfCount}
          onOpenPage={onOpenPage}
          onOpenNotifications={onOpenNotifications}
          onOpenMessages={onOpenMessages}
          onWindowDragStart={onWindowDragStart}
          onTuck={onTuck}
        />

        <div
          ref={panelRef}
          id="island-panel"
          className="island__expanded"
          aria-hidden={!isExpanded}
          inert={!isExpanded}
        >
          <IslandHeader
            page={page}
            features={features}
            pageOrder={pageOrder}
            onPageOrderChange={onPageOrderChange}
            showTitle={showTitle}
            isDetached={isDetached}
            qqNotificationCount={qqNotificationCount}
            wechatPending={wechatPending}
            onPageChange={onPageChange}
            onCollapse={onCollapse}
            onOpenSettings={onOpenSettings}
            onOpenMessages={onOpenMessages}
            onOpenTranslator={onOpenTranslator}
            onResetPosition={onResetPosition}
            minimizesToTray={minimizesToTray}
            onMinimize={onMinimize}
          />
          <div ref={contentRef} className="island__content">{children}</div>
        </div>
      </div>
    </section>
  );
}

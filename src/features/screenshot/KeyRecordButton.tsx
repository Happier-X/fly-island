import { useEffect, useRef, useState } from "react";
import { isModifierKey, vkFromCode } from "./keys";
import "./KeyRecordButton.css";

/**
 * The chip that records one key of the screenshot overlay.
 *
 * While recording it reads keys from the whole window (same reason as
 * `ShortcutRecordButton`: clicking into a window that was not active moves the focus).
 * Escape cancels, Backspace clears the binding, any other key binds it.
 */
export function KeyRecordButton({
  name,
  recordingLabel,
  unboundLabel,
  label,
  disabled,
  onRecord,
}: {
  /** What the overlay calls this key; empty when nothing is bound. */
  name: string;
  recordingLabel: string;
  unboundLabel: string;
  /** Said aloud, since the chip itself only shows the key. */
  label: string;
  disabled?: boolean;
  onRecord: (vk: number) => void;
}) {
  const [recording, setRecording] = useState(false);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const latest = useRef(onRecord);
  useEffect(() => {
    latest.current = onRecord;
  });

  useEffect(() => {
    if (!recording) return;
    buttonRef.current?.focus();
    const swallow = (event: KeyboardEvent) => {
      event.preventDefault();
      event.stopPropagation();
    };
    const keyDown = (event: KeyboardEvent) => {
      swallow(event);
      if (event.key === "Escape") {
        setRecording(false);
        return;
      }
      if (isModifierKey(event.key)) return;
      setRecording(false);
      if (event.key === "Backspace") {
        latest.current(0);
        return;
      }
      const vk = vkFromCode(event.code);
      if (vk !== null) latest.current(vk);
    };
    const pointerDown = (event: PointerEvent) => {
      if (!(event.target instanceof Node) || !buttonRef.current?.contains(event.target)) {
        setRecording(false);
      }
    };
    window.addEventListener("keydown", keyDown, true);
    window.addEventListener("pointerdown", pointerDown, true);
    return () => {
      window.removeEventListener("keydown", keyDown, true);
      window.removeEventListener("pointerdown", pointerDown, true);
    };
  }, [recording]);

  useEffect(() => {
    if (disabled) setRecording(false);
  }, [disabled]);

  return (
    <button
      ref={buttonRef}
      type="button"
      className={`key-record-button${recording ? " key-record-button--recording" : ""}${
        name ? "" : " key-record-button--empty"
      }`}
      disabled={disabled}
      aria-pressed={recording}
      aria-label={`${label}：${name || unboundLabel}`}
      onClick={() => setRecording(true)}
    >
      {recording ? recordingLabel : name || unboundLabel}
    </button>
  );
}

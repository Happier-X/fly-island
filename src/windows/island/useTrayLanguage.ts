import { useEffect } from "react";
import { useLocale } from "../../i18n";
import { setTrayLanguage } from "./api";

/** The main window stays mounted while hidden and receives settings-window locale changes. */
export function useTrayLanguage() {
  const { locale } = useLocale();
  useEffect(() => {
    void setTrayLanguage(locale).catch((error: unknown) => {
      console.error("Failed to sync tray language", error);
    });
  }, [locale]);
}

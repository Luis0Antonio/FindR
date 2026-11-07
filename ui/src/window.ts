import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

export const currentWindow = getCurrentWebviewWindow();
export const WINDOW = currentWindow.label;
export const isMainWindow = WINDOW === "main";

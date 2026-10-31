import { api, type FileEntry } from "./api";
import { useApp, type PaneId } from "./state/panes";

const IMAGE_TARGETS = ["png", "jpeg", "heic", "webp", "tiff"];
const VIDEO_TARGETS = ["mp4", "mov", "mp3", "m4a"];
const AUDIO_TARGETS = ["mp3", "m4a", "wav"];

const GROUPS: Record<string, string[]> = {
  ...Object.fromEntries(["png", "jpg", "jpeg", "heic", "heif", "webp", "tiff", "tif", "gif", "bmp", "avif"].map((e) => [e, IMAGE_TARGETS])),
  ...Object.fromEntries(["mov", "mp4", "m4v"].map((e) => [e, VIDEO_TARGETS])),
  ...Object.fromEntries(["mp3", "m4a", "wav", "aiff", "aif", "aac", "caf", "flac"].map((e) => [e, AUDIO_TARGETS])),
};

const SAME_FORMAT: Record<string, string> = { jpg: "jpeg", tif: "tiff", heif: "heic", aif: "aiff" };

function format(entry: FileEntry): string | null {
  const ext = entry.ext?.toLowerCase();
  return !entry.isDir && ext ? (SAME_FORMAT[ext] ?? ext) : null;
}

/** Formats every entry can be converted to, minus the format they're all already in. */
export function convertTargets(entries: FileEntry[]): string[] {
  const formats = entries.map(format);
  const groups = formats.map((f) => (f ? GROUPS[f] : undefined));
  if (!groups.length || groups.some((g) => g !== groups[0]) || !groups[0]) return [];
  return groups[0].filter((t) => !formats.every((f) => f === t));
}

export async function convertEntries(id: PaneId, entries: FileEntry[], target: string) {
  const todo = entries.filter((e) => format(e) !== target);
  const created: string[] = [];
  const errors: string[] = [];
  for (const [i, entry] of todo.entries()) {
    const counter = todo.length > 1 ? ` (${i + 1} of ${todo.length})` : "";
    useApp.setState({ activity: `Converting “${entry.name}” to .${target}${counter}…` });
    try {
      created.push(await api.convertFile(entry.path, target));
    } catch (e) {
      errors.push(String((e as { message?: string }).message ?? e));
    }
  }
  useApp.setState({ activity: null, notice: errors.length ? errors[0] + (errors.length > 1 ? ` (and ${errors.length - 1} more)` : "") : null });
  const s = useApp.getState();
  await s.reload(id);
  if (created.length) s.patchPane(id, { selection: new Set(created), cursor: created[0], anchor: created[0] });
}

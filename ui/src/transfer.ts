import { api, type ConflictPolicy, type OpResult } from "./api";
import { baseName, parentOf } from "./format";
import { useApp, type PaneId } from "./state/panes";

let opCounter = 0;

function askConflict(name: string, remaining: number) {
  return new Promise<{ policy: ConflictPolicy; all: boolean }>((resolve) =>
    useApp.setState({
      conflictPrompt: {
        name,
        remaining,
        resolve: (choice) => {
          useApp.setState({ conflictPrompt: null });
          resolve(choice);
        },
      },
    }),
  );
}

/** Moves (or copies) `sources` into `target`, resolving name conflicts with the user. */
export async function transfer(sources: string[], target: string, copy: boolean) {
  const opId = `op-${++opCounter}`;
  const merge = (into: OpResult, r: OpResult) => {
    into.done.push(...r.done);
    into.errors.push(...r.errors);
  };
  const result: OpResult = { done: [], conflicts: [], errors: [] };
  try {
    const first = await api.moveItems(sources, target, copy, null, opId);
    merge(result, first);
    const groups: Record<ConflictPolicy, string[]> = { replace: [], keepBoth: [], skip: [] };
    let applyAll: ConflictPolicy | null = null;
    for (let i = 0; i < first.conflicts.length; i++) {
      const c = first.conflicts[i];
      let policy: ConflictPolicy | null = applyAll;
      if (!policy) {
        const choice = await askConflict(baseName(c.target), first.conflicts.length - i - 1);
        policy = choice.policy;
        if (choice.all) applyAll = policy;
      }
      groups[policy].push(c.source);
    }
    for (const policy of ["replace", "keepBoth"] as const) {
      if (groups[policy].length) merge(result, await api.moveItems(groups[policy], target, copy, policy, opId));
    }
  } catch (e) {
    result.errors.push({ path: target, message: String((e as { message?: string }).message ?? e) });
  }
  useApp.setState({ progress: null });
  if (result.errors.length) {
    const first = result.errors[0];
    const more = result.errors.length > 1 ? ` (and ${result.errors.length - 1} more)` : "";
    useApp.setState({ notice: `Couldn't ${copy ? "copy" : "move"} “${baseName(first.path)}”: ${first.message}${more}` });
  }
  await afterTransfer(sources, target, result.done);
}

async function afterTransfer(sources: string[], target: string, done: string[]) {
  const s = useApp.getState();
  const touched = new Set([target, ...sources.map((p) => parentOf(p) ?? p)]);
  for (const id of ["left", "right"] as PaneId[]) {
    if (id === "right" && !s.splitEnabled) continue;
    const pane = s.panes[id];
    if (!touched.has(pane.path)) continue;
    await s.reload(id);
    if (pane.path === target && done.length) {
      useApp.getState().patchPane(id, { selection: new Set(done), cursor: done[0], anchor: done[0] });
    }
  }
}

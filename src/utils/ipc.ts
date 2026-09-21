

import { invoke } from '@tauri-apps/api/core';


export async function callCommand<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (err) {
    throw new Error(`[ipc:${cmd}] ${String(err)}`);
  }
}

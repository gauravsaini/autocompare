/// <reference types="vite/client" />

declare global {
  interface ImportMetaEnv {
    /** When `"1"`, the IPC client serves in-memory fixtures instead of Tauri. */
    readonly VITE_MOCK_IPC?: string;
  }
}

export {};

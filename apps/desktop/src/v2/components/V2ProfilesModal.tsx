import React, { useEffect, useState } from "react";
import { Globe, Minus, Plus } from "lucide-react";
import V2Modal from "./V2Modal";
import {
  DEFAULT_VFS_PROFILES,
  loadVfsProfiles,
  saveVfsProfiles,
  type VfsProfileEntry,
} from "../utils/formatsAndProfiles";

export interface V2ProfilesModalProps {
  readonly open: boolean;
  readonly onClose: () => void;
  readonly onSaved?: (msg: string) => void;
}

type ProfileTab = "global" | "vfs" | "connection" | "transfer";

const BUILTIN_VFS_SCHEMES = [
  { scheme: "file://", desc: "Local filesystem (read/write, symlink-safe atomic writes)" },
  { scheme: "zip://", desc: "Pure-Rust Zip archive reader with #subpath support" },
  { scheme: "tar:// / tgz://", desc: "Pure-Rust Tar & Tar.gz archive reader" },
  { scheme: "bcss://", desc: "Beyond Compare Session Snapshot (.bcss) frozen tree reader" },
] as const;

export default function V2ProfilesModal({
  open,
  onClose,
  onSaved,
}: V2ProfilesModalProps): React.JSX.Element {
  const [profiles, setProfiles] = useState<VfsProfileEntry[]>(() => loadVfsProfiles());
  const [selectedId, setSelectedId] = useState<string>(() => profiles[0]?.id ?? "default");
  const [tab, setTab] = useState<ProfileTab>("global");

  useEffect(() => {
    if (!open) return;
    const loaded = loadVfsProfiles();
    setProfiles(loaded);
    setSelectedId(loaded[0]?.id ?? "default");
    setTab("global");
  }, [open]);

  const selected = profiles.find((p) => p.id === selectedId) ?? profiles[0];

  const updateSelected = (patch: Partial<VfsProfileEntry>): void => {
    if (!selected) return;
    setProfiles((prev) => prev.map((p) => (p.id === selected.id ? { ...p, ...patch } : p)));
  };

  const addProfile = (): void => {
    const id = `profile-${Date.now()}`;
    const base = DEFAULT_VFS_PROFILES[0];
    setProfiles((prev) => [...prev, { ...base, id, name: `Profile ${prev.length}` }]);
    setSelectedId(id);
  };

  const removeSelected = (): void => {
    if (!selected || selected.id === "default") return;
    const next = profiles.filter((p) => p.id !== selected.id);
    setProfiles(next);
    setSelectedId(next[0]?.id ?? "default");
  };

  return (
    <V2Modal
      open={open}
      title="Profiles"
      subtitle="VFS connection defaults, SSH/SSL credentials, and ASCII transfer masks"
      widthClass="max-w-3xl"
      onClose={onClose}
      footer={
        <div className="flex w-full items-center justify-between gap-2">
          <button
            type="button"
            onClick={() => {
              setProfiles([...DEFAULT_VFS_PROFILES]);
              setSelectedId("default");
            }}
            className="v2-btn"
          >
            Factory Defaults
          </button>
          <div className="flex items-center gap-2">
            <button type="button" onClick={onClose} className="v2-btn">
              Cancel
            </button>
            <button
              type="button"
              onClick={() => {
                saveVfsProfiles(profiles);
                onSaved?.(`Saved ${profiles.length} VFS profile(s)`);
                onClose();
              }}
              className="v2-btn v2-btn-primary"
            >
              Save
            </button>
          </div>
        </div>
      }
    >
      <div data-testid="v2/profiles-modal" className="grid h-[340px] grid-cols-[190px_1fr] gap-3 text-[12px]">
        {/* Left Sidebar: Profile List */}
        <div className="flex min-h-0 flex-col overflow-hidden rounded-xl border border-[var(--v2-border)] bg-[var(--v2-bg)]/70">
          <div className="min-h-0 flex-1 overflow-y-auto p-1">
            {profiles.map((prof) => {
              const active = prof.id === selected?.id;
              return (
                <button
                  key={prof.id}
                  type="button"
                  onClick={() => setSelectedId(prof.id)}
                  className={`flex w-full items-center gap-1.5 rounded-[6px] px-2 py-1.5 text-left text-[11.5px] ${
                    active
                      ? "bg-[var(--v2-accent-soft)] font-medium text-[var(--v2-accent)]"
                      : "hover:bg-[var(--v2-btn-hover)]"
                  }`}
                >
                  <Globe size={12} className="shrink-0" />
                  <span className="truncate">{prof.name}</span>
                </button>
              );
            })}
          </div>
          <div className="flex items-center gap-1 border-t border-[var(--v2-border)] p-1.5">
            <button type="button" title="New profile" onClick={addProfile} className="v2-btn h-[22px] px-1.5">
              <Plus size={12} />
            </button>
            <button
              type="button"
              title="Remove profile"
              disabled={selected?.id === "default"}
              onClick={removeSelected}
              className="v2-btn h-[22px] px-1.5"
            >
              <Minus size={12} />
            </button>
          </div>
        </div>

        {/* Right Tabbed Editor */}
        {selected ? (
          <div className="flex min-h-0 flex-col overflow-hidden rounded-xl border border-[var(--v2-border)] bg-[var(--v2-surface)]">
            <div className="flex items-center gap-1 border-b border-[var(--v2-border)] bg-[var(--v2-bg)]/70 px-2.5 py-2">
              {(
                [
                  ["global", "Global"],
                  ["vfs", "VFS Schemes"],
                  ["connection", "Connection"],
                  ["transfer", "Transfer"],
                ] as const
              ).map(([id, label]) => (
                <button
                  key={id}
                  type="button"
                  onClick={() => setTab(id)}
                  className={`rounded-lg px-2.5 py-1 text-[11.5px] font-medium transition-colors ${
                    tab === id
                      ? "bg-[var(--v2-accent-fill)] text-[var(--v2-accent-fg)] shadow-sm"
                      : "text-[var(--v2-fg-muted)] hover:bg-[var(--v2-surface-subtle)] hover:text-[var(--v2-fg)]"
                  }`}
                >
                  {label}
                </button>
              ))}
            </div>

            <div className="min-h-0 flex-1 space-y-3 overflow-y-auto p-3">
              {tab === "global" ? (
                <>
                  {selected.id !== "default" ? (
                    <label className="block">
                      <span className="mb-1 block text-[11px] font-medium text-[var(--v2-fg-muted)]">
                        Profile Name
                      </span>
                      <input
                        type="text"
                        value={selected.name}
                        onChange={(e) => updateSelected({ name: e.target.value })}
                        className="v2-input w-full"
                      />
                    </label>
                  ) : null}
                  <label className="block">
                    <span className="mb-1 block text-[11px] font-medium text-[var(--v2-fg-muted)]">
                      Email address for anonymous logins
                    </span>
                    <input
                      type="text"
                      value={selected.anonymousEmail}
                      onChange={(e) => updateSelected({ anonymousEmail: e.target.value })}
                      className="v2-input w-full"
                    />
                  </label>
                  <label className="block">
                    <span className="mb-1 block text-[11px] font-medium text-[var(--v2-fg-muted)]">
                      SSH private key file
                    </span>
                    <input
                      type="text"
                      value={selected.sshPrivateKeyPath}
                      onChange={(e) => updateSelected({ sshPrivateKeyPath: e.target.value })}
                      placeholder="~/.ssh/id_ed25519"
                      className="v2-input v2-mono w-full"
                    />
                  </label>
                  <label className="block">
                    <span className="mb-1 block text-[11px] font-medium text-[var(--v2-fg-muted)]">
                      ASCII types (one mask per line)
                    </span>
                    <textarea
                      rows={4}
                      value={selected.asciiTypes.join("\n")}
                      onChange={(e) =>
                        updateSelected({
                          asciiTypes: e.target.value
                            .split(/\r?\n/)
                            .map((s) => s.trim())
                            .filter(Boolean),
                        })
                      }
                      className="v2-input v2-mono h-auto w-full py-1.5 text-[11px]"
                    />
                  </label>
                </>
              ) : null}

              {tab === "vfs" ? (
                <div className="space-y-2">
                  <p className="text-[11px] text-[var(--v2-fg-muted)]">
                    Registered schemes in <code className="v2-mono">crates/vfs/src/registry.rs</code>:
                  </p>
                  {BUILTIN_VFS_SCHEMES.map((item) => (
                    <div
                      key={item.scheme}
                      className="rounded-[6px] border border-[var(--v2-border)] bg-[var(--v2-surface-subtle)] px-2.5 py-2"
                    >
                      <div className="v2-mono text-[11.5px] font-semibold text-[var(--v2-accent)]">
                        {item.scheme}
                      </div>
                      <div className="text-[11px] text-[var(--v2-fg-muted)]">{item.desc}</div>
                    </div>
                  ))}
                </div>
              ) : null}

              {tab === "connection" ? (
                <div className="space-y-3">
                  <label className="block">
                    <span className="mb-1 block text-[11px] font-medium text-[var(--v2-fg-muted)]">
                      Connection Timeout (seconds)
                    </span>
                    <input
                      type="number"
                      min={5}
                      max={300}
                      value={selected.timeoutSecs}
                      onChange={(e) =>
                        updateSelected({ timeoutSecs: Math.max(5, Number(e.target.value) || 30) })
                      }
                      className="v2-input w-28"
                    />
                  </label>
                  <label className="block">
                    <span className="mb-1 block text-[11px] font-medium text-[var(--v2-fg-muted)]">
                      SSL client certificate
                    </span>
                    <input
                      type="text"
                      value={selected.sslClientCertPath}
                      onChange={(e) => updateSelected({ sslClientCertPath: e.target.value })}
                      placeholder="Optional PEM path"
                      className="v2-input v2-mono w-full"
                    />
                  </label>
                </div>
              ) : null}

              {tab === "transfer" ? (
                <div className="space-y-3">
                  <label className="block">
                    <span className="mb-1 block text-[11px] font-medium text-[var(--v2-fg-muted)]">
                      Default Transfer Mode
                    </span>
                    <select
                      value={selected.transferMode}
                      onChange={(e) =>
                        updateSelected({
                          transferMode: e.target.value as VfsProfileEntry["transferMode"],
                        })
                      }
                      className="v2-input w-48"
                    >
                      <option value="auto">Auto (use ASCII types)</option>
                      <option value="binary">Binary</option>
                      <option value="ascii">ASCII</option>
                    </select>
                  </label>
                  <label className="flex items-center gap-2 text-[11.5px]">
                    <input
                      type="checkbox"
                      checked={selected.preserveTimestamps}
                      onChange={(e) => updateSelected({ preserveTimestamps: e.target.checked })}
                      className="accent-[var(--v2-accent)]"
                    />
                    Preserve modified timestamps during copy/sync
                  </label>
                </div>
              ) : null}
            </div>
          </div>
        ) : null}
      </div>
    </V2Modal>
  );
}

export type SessionKind =
  | "home"
  | "text"
  | "folder"
  | "table"
  | "hex"
  | "merge"
  | "folderMerge"
  | "sync"
  | "media";

export type FilterMode = "all" | "diffs" | "same";

export interface V2Tab {
  readonly id: string;
  readonly kind: SessionKind;
  readonly title: string;
  readonly leftPath: string;
  readonly rightPath: string;
  readonly dirty?: boolean;
}

export interface V2StatusInfo {
  readonly left: string;
  readonly center: string;
  readonly right: string;
}

export interface V2ViewCommands {
  readonly filterMode?: FilterMode;
  readonly setFilterMode?: (mode: FilterMode) => void;
  readonly contextEnabled?: boolean;
  readonly toggleContext?: () => void;
  readonly minorEnabled?: boolean;
  readonly toggleMinor?: () => void;
  readonly openRules?: () => void;
  readonly openFormat?: () => void;
  readonly openFind?: () => void;
  readonly copySelected?: () => void;
  readonly copySelectedRight?: () => void;
  readonly canCopy?: boolean;
  readonly prevSection?: () => void;
  readonly nextSection?: () => void;
  readonly hasSections?: boolean;
  readonly swapSides?: () => void;
  readonly reload?: () => void;
}

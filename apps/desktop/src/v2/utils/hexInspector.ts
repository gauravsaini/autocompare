export interface HexInspectorValues {
  readonly uint8: number | null;
  readonly int8: number | null;
  readonly uint16: number | null;
  readonly int16: number | null;
  readonly uint32: number | null;
  readonly int32: number | null;
  readonly float32: string | null;
  readonly binary: string | null;
  readonly ascii: string | null;
}

/**
 * Converts a UTF-8 / ASCII text string into space-separated uppercase hex
 * bytes suitable for `hexFind`.
 */
export function asciiToHexQuery(text: string): string {
  if (!text) return "";
  const bytes = new TextEncoder().encode(text);
  return Array.from(bytes, (b) => b.toString(16).padStart(2, "0").toUpperCase()).join(" ");
}

/**
 * Formats an array of byte values (0..255) as space-separated uppercase hex.
 */
export function formatBytesAsHex(bytes: readonly number[]): string {
  return bytes.map((b) => (b & 0xff).toString(16).padStart(2, "0").toUpperCase()).join(" ");
}

/**
 * Formats an array of byte values as printable ASCII (non-printable as `.`).
 */
export function formatBytesAsAscii(bytes: readonly number[]): string {
  return bytes
    .map((b) => {
      const v = b & 0xff;
      return v >= 0x20 && v <= 0x7e ? String.fromCharCode(v) : ".";
    })
    .join("");
}

/**
 * Decodes up to 4 bytes starting at index 0 into common integer and float
 * representations with selectable endianness.
 */
export function decodeHexInspector(
  bytes: readonly number[],
  littleEndian = true,
): HexInspectorValues {
  if (bytes.length === 0) {
    return {
      uint8: null,
      int8: null,
      uint16: null,
      int16: null,
      uint32: null,
      int32: null,
      float32: null,
      binary: null,
      ascii: null,
    };
  }

  const buf = new ArrayBuffer(Math.max(4, bytes.length));
  const u8 = new Uint8Array(buf);
  for (let i = 0; i < Math.min(bytes.length, 4); i += 1) {
    u8[i] = bytes[i] & 0xff;
  }
  const view = new DataView(buf);

  const b0 = u8[0];
  const uint8 = view.getUint8(0);
  const int8 = view.getInt8(0);
  const uint16 = bytes.length >= 2 ? view.getUint16(0, littleEndian) : null;
  const int16 = bytes.length >= 2 ? view.getInt16(0, littleEndian) : null;
  const uint32 = bytes.length >= 4 ? view.getUint32(0, littleEndian) : null;
  const int32 = bytes.length >= 4 ? view.getInt32(0, littleEndian) : null;
  let float32: string | null = null;
  if (bytes.length >= 4) {
    const f = view.getFloat32(0, littleEndian);
    float32 = Number.isFinite(f) ? Number(f.toPrecision(6)).toString() : String(f);
  }

  return {
    uint8,
    int8,
    uint16,
    int16,
    uint32,
    int32,
    float32,
    binary: b0.toString(2).padStart(8, "0"),
    ascii: b0 >= 0x20 && b0 <= 0x7e ? String.fromCharCode(b0) : "·",
  };
}

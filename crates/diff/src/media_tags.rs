//! Pure media tag reader and difference analysis for FLAC, MP3, and MP4/M4A audio.
//!
//! Layer 0: pure safe Rust, no I/O, no panics, no unsafe code. Hostile inputs
//! (truncated headers, corrupt chunk lengths, arbitrary fuzz bytes) are handled
//! safely and return `None` or honest metadata without aborting.

/// Container format detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MediaFormat {
    #[default]
    Unknown,
    Flac,
    Mp3,
    Mp4,
    Aac,
}

/// Normalized metadata tags read from an audio file.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MediaTags {
    /// Track title.
    pub title: Option<String>,
    /// Primary recording artist.
    pub artist: Option<String>,
    /// Album title.
    pub album: Option<String>,
    /// Album artist (if distinct from track artist).
    pub album_artist: Option<String>,
    /// Release year or date string.
    pub year: Option<String>,
    /// Track number.
    pub track: Option<u32>,
    /// Total tracks on album.
    pub track_total: Option<u32>,
    /// Disc number.
    pub disc: Option<u32>,
    /// Total discs in the release set.
    pub disc_total: Option<u32>,
    /// Genre description.
    pub genre: Option<String>,
    /// User or release comment.
    pub comment: Option<String>,
    /// Container format detected.
    pub format: MediaFormat,
    /// All raw or custom key-value tag pairs found.
    pub extra_tags: Vec<(String, String)>,
}

/// Detailed difference between two sets of media tags.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaTagsDiff {
    /// Fields matching with identical values: `(field_name, value)`.
    pub matching: Vec<(String, String)>,
    /// Fields present on both sides but with differing values: `(field_name, left_val, right_val)`.
    pub differing: Vec<(String, String, String)>,
    /// Fields present only on the left side: `(field_name, value)`.
    pub left_only: Vec<(String, String)>,
    /// Fields present only on the right side: `(field_name, value)`.
    pub right_only: Vec<(String, String)>,
    /// True when both tag sets are completely identical.
    pub identical: bool,
}

/// Reads media tags by sniffing container type (FLAC, MP3, MP4/M4A, or AAC).
pub fn read_media_tags(bytes: &[u8]) -> Option<MediaTags> {
    if bytes.len() < 4 {
        return None;
    }

    if bytes.get(0..4) == Some(b"fLaC") {
        return read_flac_tags(bytes);
    }

    if bytes.get(4..8) == Some(b"ftyp") || bytes.get(0..4) == Some(b"ftyp") {
        return read_mp4_tags(bytes);
    }

    if is_aac_adts(bytes) {
        return read_aac_tags(bytes);
    }

    if bytes.get(0..3) == Some(b"ID3") {
        let aac_tags = read_aac_tags(bytes).filter(|aac| {
            aac.extra_tags
                .iter()
                .any(|(k, _)| k == "Codec" || k == "Sample Rate")
        });
        if let Some(aac) = aac_tags {
            return Some(aac);
        }
        return read_mp3_tags(bytes);
    }

    // Try each decoder as fallback
    read_flac_tags(bytes)
        .or_else(|| read_mp3_tags(bytes))
        .or_else(|| read_mp4_tags(bytes))
        .or_else(|| read_aac_tags(bytes))
}

fn is_aac_adts(bytes: &[u8]) -> bool {
    bytes.first() == Some(&0xFF) && bytes.get(1).map(|&b| (b & 0xF6) == 0xF0).unwrap_or(false)
}

/// Reads AAC metadata tags from ADTS streams or AAC files with ID3 tags.
pub fn read_aac_tags(bytes: &[u8]) -> Option<MediaTags> {
    if bytes.len() < 4 {
        return None;
    }
    let mut tags = MediaTags {
        format: MediaFormat::Aac,
        ..Default::default()
    };
    let mut found_any = false;

    let mut pos = 0usize;
    if bytes.len() >= 10 && bytes.get(0..3) == Some(b"ID3") {
        let version = bytes.get(3).copied().unwrap_or(3);
        let b0 = bytes.get(6).copied().unwrap_or(0) as usize & 0x7F;
        let b1 = bytes.get(7).copied().unwrap_or(0) as usize & 0x7F;
        let b2 = bytes.get(8).copied().unwrap_or(0) as usize & 0x7F;
        let b3 = bytes.get(9).copied().unwrap_or(0) as usize & 0x7F;
        let tag_size = (b0 << 21) | (b1 << 14) | (b2 << 7) | b3;
        let end = 10usize.saturating_add(tag_size).min(bytes.len());
        if let Some(tag_bytes) = bytes.get(10..end) {
            parse_id3v2_frames(tag_bytes, version, &mut tags);
            found_any = true;
        }
        pos = end;
    }

    let adts_b1 = match (bytes.get(pos), bytes.get(pos.saturating_add(1))) {
        (Some(&b0), Some(&b1)) if b0 == 0xFF && (b1 & 0xF6) == 0xF0 => Some(b1),
        _ => None,
    };
    if let Some(b1) = adts_b1 {
        let id = (b1 >> 3) & 1;
        let codec = if id == 0 { "MPEG-4 AAC" } else { "MPEG-2 AAC" };
        tags.extra_tags
            .push(("Codec".to_string(), codec.to_string()));

        if let Some(&b2) = bytes.get(pos.saturating_add(2)) {
            let freq_idx = (b2 >> 2) & 0x0F;
            let sample_rate = match freq_idx {
                0 => Some(96000),
                1 => Some(88200),
                2 => Some(64000),
                3 => Some(48000),
                4 => Some(44100),
                5 => Some(32000),
                6 => Some(24000),
                7 => Some(22050),
                8 => Some(16000),
                9 => Some(12000),
                10 => Some(11025),
                11 => Some(8000),
                12 => Some(7350),
                _ => None,
            };
            if let Some(sr) = sample_rate {
                tags.extra_tags
                    .push(("Sample Rate".to_string(), format!("{sr} Hz")));
            }
            let ch_high = (b2 & 1) << 2;
            if let Some(&b3) = bytes.get(pos.saturating_add(3)) {
                let ch_low = (b3 >> 6) & 3;
                let channels = ch_high | ch_low;
                tags.extra_tags
                    .push(("Channels".to_string(), channels.to_string()));
            }
        }
        found_any = true;
    }

    if found_any {
        Some(tags)
    } else {
        None
    }
}

/// Reads FLAC metadata blocks and extracts Vorbis comments.
pub fn read_flac_tags(bytes: &[u8]) -> Option<MediaTags> {
    if bytes.get(0..4) != Some(b"fLaC") {
        return None;
    }

    let mut tags = MediaTags {
        format: MediaFormat::Flac,
        ..Default::default()
    };

    let mut pos = 4usize;
    while pos.saturating_add(4) <= bytes.len() {
        let header_byte = bytes.get(pos).copied()?;
        let is_last = (header_byte & 0x80) != 0;
        let block_type = header_byte & 0x7F;

        let b1 = usize::from(bytes.get(pos + 1).copied()?);
        let b2 = usize::from(bytes.get(pos + 2).copied()?);
        let b3 = usize::from(bytes.get(pos + 3).copied()?);
        let length = (b1 << 16) | (b2 << 8) | b3;

        let data_start = pos.checked_add(4)?;
        let data_end = data_start.checked_add(length)?;
        if data_end > bytes.len() {
            break;
        }

        let block_data = bytes.get(data_start..data_end)?;
        if block_type == 4 {
            // VORBIS_COMMENT block
            parse_vorbis_comment(block_data, &mut tags);
        }

        pos = data_end;
        if is_last {
            break;
        }
    }

    Some(tags)
}

fn parse_vorbis_comment(data: &[u8], tags: &mut MediaTags) {
    if data.len() < 8 {
        return;
    }

    let vendor_len = match data.get(0..4) {
        Some(s) => u32::from_le_bytes(s.try_into().unwrap_or([0; 4])) as usize,
        None => return,
    };

    let mut pos = match 4usize.checked_add(vendor_len) {
        Some(p) if p <= data.len() => p,
        _ => return,
    };

    let user_comment_count = match data.get(pos..pos + 4) {
        Some(s) => u32::from_le_bytes(s.try_into().unwrap_or([0; 4])) as usize,
        None => return,
    };
    pos = match pos.checked_add(4) {
        Some(p) => p,
        None => return,
    };

    let safe_count = user_comment_count.min(2048);
    for _ in 0..safe_count {
        if pos.saturating_add(4) > data.len() {
            break;
        }
        let comment_len = match data.get(pos..pos + 4) {
            Some(s) => u32::from_le_bytes(s.try_into().unwrap_or([0; 4])) as usize,
            None => break,
        };
        pos = match pos.checked_add(4) {
            Some(p) => p,
            None => break,
        };

        let next_pos = match pos.checked_add(comment_len) {
            Some(p) if p <= data.len() => p,
            _ => break,
        };

        if let Some(comment_bytes) = data.get(pos..next_pos) {
            let entry = String::from_utf8_lossy(comment_bytes);
            if let Some((key, val)) = entry.split_once('=') {
                apply_tag_key_value(tags, key.trim(), val.trim());
            }
        }
        pos = next_pos;
    }
}

/// Reads MP3 ID3v2 and ID3v1 metadata tags.
pub fn read_mp3_tags(bytes: &[u8]) -> Option<MediaTags> {
    let mut tags = MediaTags {
        format: MediaFormat::Mp3,
        ..Default::default()
    };
    let mut found_any = false;

    // Check ID3v2 at start
    if bytes.len() >= 10 && bytes.get(0..3) == Some(b"ID3") {
        let version = bytes.get(3).copied().unwrap_or(3);
        let b0 = bytes.get(6).copied().unwrap_or(0) as usize & 0x7F;
        let b1 = bytes.get(7).copied().unwrap_or(0) as usize & 0x7F;
        let b2 = bytes.get(8).copied().unwrap_or(0) as usize & 0x7F;
        let b3 = bytes.get(9).copied().unwrap_or(0) as usize & 0x7F;
        let tag_size = (b0 << 21) | (b1 << 14) | (b2 << 7) | b3;

        let end = 10usize.saturating_add(tag_size).min(bytes.len());
        if let Some(tag_bytes) = bytes.get(10..end) {
            parse_id3v2_frames(tag_bytes, version, &mut tags);
            found_any = true;
        }
    }

    // Check ID3v1 at tail (128 bytes)
    if bytes.len() >= 128 {
        let tail = bytes.get(bytes.len() - 128..).unwrap_or(&[]);
        if tail.get(0..3) == Some(b"TAG") {
            parse_id3v1(tail, &mut tags);
            found_any = true;
        }
    }

    if found_any {
        Some(tags)
    } else {
        None
    }
}

fn parse_id3v2_frames(data: &[u8], version: u8, tags: &mut MediaTags) {
    let mut pos = 0usize;
    let header_size = if version == 2 { 6 } else { 10 };

    while pos.saturating_add(header_size) <= data.len() {
        let id_bytes = match data.get(pos..pos + (if version == 2 { 3 } else { 4 })) {
            Some(b) => b,
            None => break,
        };

        // Stop on null padding
        if id_bytes.first() == Some(&0) {
            break;
        }

        let frame_id = String::from_utf8_lossy(id_bytes).to_string();

        let frame_size = if version == 2 {
            let b0 = usize::from(data.get(pos + 3).copied().unwrap_or(0));
            let b1 = usize::from(data.get(pos + 4).copied().unwrap_or(0));
            let b2 = usize::from(data.get(pos + 5).copied().unwrap_or(0));
            (b0 << 16) | (b1 << 8) | b2
        } else if version == 4 {
            // Synchsafe u32 in v2.4
            let b0 = usize::from(data.get(pos + 4).copied().unwrap_or(0) & 0x7F);
            let b1 = usize::from(data.get(pos + 5).copied().unwrap_or(0) & 0x7F);
            let b2 = usize::from(data.get(pos + 6).copied().unwrap_or(0) & 0x7F);
            let b3 = usize::from(data.get(pos + 7).copied().unwrap_or(0) & 0x7F);
            (b0 << 21) | (b1 << 14) | (b2 << 7) | b3
        } else {
            // Regular u32 in v2.3
            let b = match data.get(pos + 4..pos + 8) {
                Some(slice) => slice,
                None => break,
            };
            u32::from_be_bytes(b.try_into().unwrap_or([0; 4])) as usize
        };

        let body_start = match pos.checked_add(header_size) {
            Some(p) => p,
            None => break,
        };
        let body_end = match body_start.checked_add(frame_size) {
            Some(p) if p <= data.len() => p,
            _ => break,
        };

        if let Some(body) = data.get(body_start..body_end).filter(|b| !b.is_empty()) {
            let text = decode_id3_text(body);
            if !text.is_empty() {
                apply_id3_frame(tags, &frame_id, &text);
            }
        }

        pos = body_end;
    }
}

fn decode_id3_text(body: &[u8]) -> String {
    let encoding = body.first().copied().unwrap_or(0);
    let payload = body.get(1..).unwrap_or(&[]);

    match encoding {
        0 => {
            // ISO-8859-1
            payload
                .iter()
                .take_while(|&&b| b != 0)
                .map(|&b| b as char)
                .collect::<String>()
                .trim()
                .to_string()
        }
        1 => {
            // UTF-16 with BOM
            decode_utf16(payload)
        }
        2 => {
            // UTF-16BE without BOM
            decode_utf16_be(payload)
        }
        3 => {
            // UTF-8
            String::from_utf8_lossy(payload)
                .trim_matches('\0')
                .trim()
                .to_string()
        }
        _ => String::from_utf8_lossy(payload).trim().to_string(),
    }
}

fn decode_utf16(bytes: &[u8]) -> String {
    if bytes.len() < 2 {
        return String::new();
    }
    let (is_le, start) = match (bytes.first(), bytes.get(1)) {
        (Some(&0xFF), Some(&0xFE)) => (true, 2),
        (Some(&0xFE), Some(&0xFF)) => (false, 2),
        _ => (true, 0),
    };
    let (chunks, _) = bytes.get(start..).unwrap_or(&[]).as_chunks::<2>();
    let u16s: Vec<u16> = chunks
        .iter()
        .map(|chunk| {
            if is_le {
                u16::from_le_bytes(*chunk)
            } else {
                u16::from_be_bytes(*chunk)
            }
        })
        .take_while(|&u| u != 0)
        .collect();
    String::from_utf16_lossy(&u16s).trim().to_string()
}

fn decode_utf16_be(bytes: &[u8]) -> String {
    let (chunks, _) = bytes.as_chunks::<2>();
    let u16s: Vec<u16> = chunks
        .iter()
        .map(|chunk| u16::from_be_bytes(*chunk))
        .take_while(|&u| u != 0)
        .collect();
    String::from_utf16_lossy(&u16s).trim().to_string()
}

fn parse_id3v1(tail: &[u8], tags: &mut MediaTags) {
    if tail.len() < 128 {
        return;
    }

    if tags.title.is_none() {
        let s = String::from_utf8_lossy(tail.get(3..33).unwrap_or(&[]))
            .trim_matches('\0')
            .trim()
            .to_string();
        if !s.is_empty() {
            tags.title = Some(s);
        }
    }
    if tags.artist.is_none() {
        let s = String::from_utf8_lossy(tail.get(33..63).unwrap_or(&[]))
            .trim_matches('\0')
            .trim()
            .to_string();
        if !s.is_empty() {
            tags.artist = Some(s);
        }
    }
    if tags.album.is_none() {
        let s = String::from_utf8_lossy(tail.get(63..93).unwrap_or(&[]))
            .trim_matches('\0')
            .trim()
            .to_string();
        if !s.is_empty() {
            tags.album = Some(s);
        }
    }
    if tags.year.is_none() {
        let s = String::from_utf8_lossy(tail.get(93..97).unwrap_or(&[]))
            .trim_matches('\0')
            .trim()
            .to_string();
        if !s.is_empty() {
            tags.year = Some(s);
        }
    }
    if tags.track.is_none() && tail.get(125) == Some(&0) {
        let track_num = tail.get(126).copied().unwrap_or(0);
        if track_num > 0 {
            tags.track = Some(u32::from(track_num));
        }
    }
}

/// Reads MP4 / M4A metadata atoms (`moov` -> `udta` -> `meta` -> `ilst`).
pub fn read_mp4_tags(bytes: &[u8]) -> Option<MediaTags> {
    if bytes.len() < 8 {
        return None;
    }

    let mut tags = MediaTags {
        format: MediaFormat::Mp4,
        ..Default::default()
    };

    if let Some(ilst_data) = find_mp4_atom_path(bytes, &[b"moov", b"udta", b"meta", b"ilst"]) {
        parse_mp4_ilst(ilst_data, &mut tags);
        Some(tags)
    } else {
        None
    }
}

fn find_mp4_atom_path<'a>(mut bytes: &'a [u8], path: &[&[u8; 4]]) -> Option<&'a [u8]> {
    let (&target, rest_path) = path.split_first()?;

    while bytes.len() >= 8 {
        let size = u32::from_be_bytes(bytes.get(0..4)?.try_into().ok()?) as usize;
        let atom_type = bytes.get(4..8)?;

        let end = if size == 0 {
            bytes.len()
        } else if size == 1 && bytes.len() >= 16 {
            u64::from_be_bytes(bytes.get(8..16)?.try_into().ok()?) as usize
        } else {
            size
        };

        if end > bytes.len() || end < 8 {
            break;
        }

        if atom_type == target {
            let offset = if target == b"meta" { 12 } else { 8 };
            let sub_bytes = bytes.get(offset.min(end)..end)?;
            if rest_path.is_empty() {
                return Some(sub_bytes);
            }
            return find_mp4_atom_path(sub_bytes, rest_path);
        }

        bytes = bytes.get(end..)?;
    }

    None
}

fn parse_mp4_ilst(data: &[u8], tags: &mut MediaTags) {
    let mut pos = 0usize;

    while pos.saturating_add(8) <= data.len() {
        let size = match data.get(pos..pos + 4) {
            Some(b) => u32::from_be_bytes(b.try_into().unwrap_or([0; 4])) as usize,
            None => break,
        };
        let atom_type = match data.get(pos + 4..pos + 8) {
            Some(b) => b,
            None => break,
        };

        let end = match pos.checked_add(size) {
            Some(p) if p <= data.len() => p,
            _ => break,
        };

        if let Some(val_str) = data.get(pos + 8..end).and_then(extract_mp4_data_string) {
            match atom_type {
                b"\xa9nam" => tags.title = Some(val_str),
                b"\xa9ART" => tags.artist = Some(val_str),
                b"aART" => tags.album_artist = Some(val_str),
                b"\xa9alb" => tags.album = Some(val_str),
                b"\xa9day" => tags.year = Some(val_str),
                b"\xa9gen" => tags.genre = Some(val_str),
                b"\xa9cmt" => tags.comment = Some(val_str),
                b"trkn" => {
                    let atom_payload = data.get(pos + 8..end).unwrap_or(&[]);
                    if let Some(track_num) = extract_mp4_track_number(atom_payload) {
                        tags.track = Some(track_num);
                    }
                    if let Some(total) = extract_mp4_track_total(atom_payload) {
                        tags.track_total = Some(total);
                    }
                }
                b"disk" => {
                    let atom_payload = data.get(pos + 8..end).unwrap_or(&[]);
                    let (disc, total) = extract_mp4_disk_numbers(atom_payload);
                    if let Some(disc) = disc {
                        tags.disc = Some(disc);
                    }
                    if let Some(total) = total {
                        tags.disc_total = Some(total);
                    }
                }
                other => {
                    let k = String::from_utf8_lossy(other).to_string();
                    tags.extra_tags.push((k, val_str));
                }
            }
        }

        pos = end;
    }
}

fn extract_mp4_data_string(payload: &[u8]) -> Option<String> {
    if payload.len() < 16 || payload.get(4..8) != Some(b"data") {
        return None;
    }
    let data_len = u32::from_be_bytes(payload.get(0..4)?.try_into().ok()?) as usize;
    let end = data_len.min(payload.len());
    let text_bytes = payload.get(16..end)?;
    Some(String::from_utf8_lossy(text_bytes).trim().to_string())
}

fn extract_mp4_track_number(payload: &[u8]) -> Option<u32> {
    if payload.len() >= 20 && payload.get(4..8) == Some(b"data") {
        let raw: [u8; 2] = payload.get(18..20)?.try_into().ok()?;
        return Some(u32::from(u16::from_be_bytes(raw)));
    }
    None
}

fn extract_mp4_track_total(payload: &[u8]) -> Option<u32> {
    if payload.len() >= 22 && payload.get(4..8) == Some(b"data") {
        let raw: [u8; 2] = payload.get(20..22)?.try_into().ok()?;
        let total = u16::from_be_bytes(raw);
        if total > 0 {
            return Some(u32::from(total));
        }
    }
    None
}

/// Reads the Apple `disk` atom value: disc number and disc total as
/// big-endian `u16` at the same offsets as `trkn`. Zero means
/// unknown and maps to `None`; a short or misshaped payload maps to
/// `(None, None)`.
fn extract_mp4_disk_numbers(payload: &[u8]) -> (Option<u32>, Option<u32>) {
    let at = |start: usize, end: usize| -> Option<u32> {
        if payload.len() < 22 || payload.get(4..8) != Some(b"data") {
            return None;
        }
        let raw: [u8; 2] = payload.get(start..end)?.try_into().ok()?;
        let val = u16::from_be_bytes(raw);
        if val > 0 {
            Some(u32::from(val))
        } else {
            None
        }
    };
    (at(18, 20), at(20, 22))
}

fn apply_tag_key_value(tags: &mut MediaTags, key: &str, val: &str) {
    if val.is_empty() {
        return;
    }
    match key.to_ascii_uppercase().as_str() {
        "TITLE" => tags.title = Some(val.to_string()),
        "ARTIST" => tags.artist = Some(val.to_string()),
        "ALBUM" => tags.album = Some(val.to_string()),
        "ALBUMARTIST" | "ALBUM ARTIST" => tags.album_artist = Some(val.to_string()),
        "DATE" | "YEAR" => tags.year = Some(val.to_string()),
        "TRACKNUMBER" | "TRACK" => {
            if let Some((num, total)) = val.split_once('/') {
                if let Ok(n) = num.trim().parse::<u32>() {
                    tags.track = Some(n);
                }
                if let Ok(t) = total.trim().parse::<u32>() {
                    tags.track_total = Some(t);
                }
            } else if let Ok(n) = val.parse::<u32>() {
                tags.track = Some(n);
            }
        }
        "TOTALTRACKS" | "TRACKTOTAL" => {
            if let Ok(n) = val.parse::<u32>() {
                tags.track_total = Some(n);
            }
        }
        "DISCNUMBER" => {
            if let Some((num, total)) = val.split_once('/') {
                if let Ok(n) = num.trim().parse::<u32>() {
                    tags.disc = Some(n);
                }
                if let Ok(t) = total.trim().parse::<u32>() {
                    tags.disc_total = Some(t);
                }
            } else if let Ok(n) = val.parse::<u32>() {
                tags.disc = Some(n);
            }
        }
        "DISCTOTAL" | "TOTALDISCS" => {
            if let Ok(n) = val.parse::<u32>() {
                tags.disc_total = Some(n);
            }
        }
        "GENRE" => tags.genre = Some(val.to_string()),
        "DESCRIPTION" | "COMMENT" => tags.comment = Some(val.to_string()),
        _ => tags.extra_tags.push((key.to_string(), val.to_string())),
    }
}

fn apply_id3_frame(tags: &mut MediaTags, frame_id: &str, text: &str) {
    match frame_id {
        "TIT2" | "TT2" => tags.title = Some(text.to_string()),
        "TPE1" | "TP1" => tags.artist = Some(text.to_string()),
        "TALB" | "TAL" => tags.album = Some(text.to_string()),
        "TPE2" | "TP2" => tags.album_artist = Some(text.to_string()),
        "TYER" | "TYE" | "TDRC" => tags.year = Some(text.to_string()),
        "TCON" | "TCO" => tags.genre = Some(text.to_string()),
        "COMM" | "COM" => tags.comment = Some(text.to_string()),
        "TRCK" | "TRK" => {
            if let Some((n, total)) = text.split_once('/') {
                if let Ok(tr) = n.trim().parse::<u32>() {
                    tags.track = Some(tr);
                }
                if let Ok(tot) = total.trim().parse::<u32>() {
                    tags.track_total = Some(tot);
                }
            } else if let Ok(tr) = text.trim().parse::<u32>() {
                tags.track = Some(tr);
            }
        }
        "TPOS" | "TPA" => {
            if let Some((d, t)) = text.split_once('/') {
                if let Ok(disc) = d.trim().parse::<u32>() {
                    tags.disc = Some(disc);
                }
                if let Ok(total) = t.trim().parse::<u32>() {
                    tags.disc_total = Some(total);
                }
            } else if let Ok(disc) = text.trim().parse::<u32>() {
                tags.disc = Some(disc);
            }
        }
        _ => tags
            .extra_tags
            .push((frame_id.to_string(), text.to_string())),
    }
}

/// Compares `extra_tags` key-value pairs by tag name.
///
/// Keys present on both sides with equal values count as matching,
/// same key with different values counts as differing, one-sided
/// keys count as left/right-only. Duplicate keys on one side are
/// compared positionally in order; surplus entries are one-sided.
/// Total: no panics, hostile keys/values pass through untouched.
fn diff_extra_tags(
    left: &MediaTags,
    right: &MediaTags,
    matching: &mut Vec<(String, String)>,
    differing: &mut Vec<(String, String, String)>,
    left_only: &mut Vec<(String, String)>,
    right_only: &mut Vec<(String, String)>,
) {
    use std::collections::{BTreeSet, HashMap};
    let mut left_by_key: HashMap<&str, Vec<&str>> = HashMap::new();
    for (k, v) in &left.extra_tags {
        left_by_key.entry(k.as_str()).or_default().push(v.as_str());
    }
    let mut right_by_key: HashMap<&str, Vec<&str>> = HashMap::new();
    for (k, v) in &right.extra_tags {
        right_by_key.entry(k.as_str()).or_default().push(v.as_str());
    }
    let mut keys: BTreeSet<&str> = BTreeSet::new();
    for k in left_by_key.keys() {
        keys.insert(k);
    }
    for k in right_by_key.keys() {
        keys.insert(k);
    }
    for key in keys {
        let l_vals = left_by_key.get(key).cloned().unwrap_or_default();
        let r_vals = right_by_key.get(key).cloned().unwrap_or_default();
        let common = l_vals.len().min(r_vals.len());
        for i in 0..common {
            let lv = l_vals.get(i).copied().unwrap_or("");
            let rv = r_vals.get(i).copied().unwrap_or("");
            if lv == rv {
                matching.push((key.to_string(), lv.to_string()));
            } else {
                differing.push((key.to_string(), lv.to_string(), rv.to_string()));
            }
        }
        for lv in l_vals.iter().skip(common) {
            left_only.push((key.to_string(), (*lv).to_string()));
        }
        for rv in r_vals.iter().skip(common) {
            right_only.push((key.to_string(), (*rv).to_string()));
        }
    }
}

/// Compares two sets of media tags and reports matches, differences, and unique fields.
pub fn diff_media_tags(left: &MediaTags, right: &MediaTags) -> MediaTagsDiff {
    let mut matching = Vec::new();
    let mut differing = Vec::new();
    let mut left_only = Vec::new();
    let mut right_only = Vec::new();

    macro_rules! compare_opt_str {
        ($field:ident, $name:expr) => {
            match (&left.$field, &right.$field) {
                (Some(l), Some(r)) if l == r => matching.push(($name.to_string(), l.clone())),
                (Some(l), Some(r)) => differing.push(($name.to_string(), l.clone(), r.clone())),
                (Some(l), None) => left_only.push(($name.to_string(), l.clone())),
                (None, Some(r)) => right_only.push(($name.to_string(), r.clone())),
                (None, None) => {}
            }
        };
    }

    compare_opt_str!(title, "Title");
    compare_opt_str!(artist, "Artist");
    compare_opt_str!(album, "Album");
    compare_opt_str!(album_artist, "Album Artist");
    compare_opt_str!(year, "Year");
    compare_opt_str!(genre, "Genre");
    compare_opt_str!(comment, "Comment");

    match (left.track, right.track) {
        (Some(l), Some(r)) if l == r => matching.push(("Track".to_string(), l.to_string())),
        (Some(l), Some(r)) => differing.push(("Track".to_string(), l.to_string(), r.to_string())),
        (Some(l), None) => left_only.push(("Track".to_string(), l.to_string())),
        (None, Some(r)) => right_only.push(("Track".to_string(), r.to_string())),
        (None, None) => {}
    }

    match (left.track_total, right.track_total) {
        (Some(l), Some(r)) if l == r => matching.push(("Track Total".to_string(), l.to_string())),
        (Some(l), Some(r)) => {
            differing.push(("Track Total".to_string(), l.to_string(), r.to_string()))
        }
        (Some(l), None) => left_only.push(("Track Total".to_string(), l.to_string())),
        (None, Some(r)) => right_only.push(("Track Total".to_string(), r.to_string())),
        (None, None) => {}
    }

    match (left.disc, right.disc) {
        (Some(l), Some(r)) if l == r => matching.push(("Disc".to_string(), l.to_string())),
        (Some(l), Some(r)) => differing.push(("Disc".to_string(), l.to_string(), r.to_string())),
        (Some(l), None) => left_only.push(("Disc".to_string(), l.to_string())),
        (None, Some(r)) => right_only.push(("Disc".to_string(), r.to_string())),
        (None, None) => {}
    }

    match (left.disc_total, right.disc_total) {
        (Some(l), Some(r)) if l == r => matching.push(("Disc Total".to_string(), l.to_string())),
        (Some(l), Some(r)) => {
            differing.push(("Disc Total".to_string(), l.to_string(), r.to_string()))
        }
        (Some(l), None) => left_only.push(("Disc Total".to_string(), l.to_string())),
        (None, Some(r)) => right_only.push(("Disc Total".to_string(), r.to_string())),
        (None, None) => {}
    }

    if left.format == right.format {
        matching.push(("Format".to_string(), format!("{:?}", left.format)));
    } else {
        differing.push((
            "Format".to_string(),
            format!("{:?}", left.format),
            format!("{:?}", right.format),
        ));
    }

    diff_extra_tags(
        left,
        right,
        &mut matching,
        &mut differing,
        &mut left_only,
        &mut right_only,
    );

    let identical = differing.is_empty() && left_only.is_empty() && right_only.is_empty();

    MediaTagsDiff {
        matching,
        differing,
        left_only,
        right_only,
        identical,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_or_garbage_returns_none() {
        assert_eq!(read_media_tags(b""), None);
        assert_eq!(read_media_tags(b"GARBAGE_BYTES_NO_MAGIC"), None);
    }

    #[test]
    fn test_synthetic_flac_vorbis_comment() {
        let mut flac_bytes = Vec::new();
        flac_bytes.extend_from_slice(b"fLaC");

        let mut comment_block = Vec::new();
        let vendor = b"reference libFLAC";
        comment_block.extend_from_slice(&(vendor.len() as u32).to_le_bytes());
        comment_block.extend_from_slice(vendor);

        // 2 comments: TITLE and ARTIST
        comment_block.extend_from_slice(&2u32.to_le_bytes());
        let c1 = b"TITLE=Song 1";
        comment_block.extend_from_slice(&(c1.len() as u32).to_le_bytes());
        comment_block.extend_from_slice(c1);
        let c2 = b"ARTIST=Cool Artist";
        comment_block.extend_from_slice(&(c2.len() as u32).to_le_bytes());
        comment_block.extend_from_slice(c2);

        // Header: is_last = 1, type = 4
        flac_bytes.push(0x84);
        let len = comment_block.len();
        flac_bytes.push(((len >> 16) & 0xFF) as u8);
        flac_bytes.push(((len >> 8) & 0xFF) as u8);
        flac_bytes.push((len & 0xFF) as u8);
        flac_bytes.extend_from_slice(&comment_block);

        let tags_opt = read_flac_tags(&flac_bytes);
        assert!(tags_opt.is_some());
        if let Some(tags) = tags_opt {
            assert_eq!(tags.title.as_deref(), Some("Song 1"));
            assert_eq!(tags.artist.as_deref(), Some("Cool Artist"));
            assert_eq!(tags.format, MediaFormat::Flac);
        }
    }

    #[test]
    fn test_synthetic_id3v1() {
        let mut mp3 = vec![0u8; 500];
        let tail_start = 500 - 128;
        if let Some(tail) = mp3.get_mut(tail_start..) {
            if let Some(s) = tail.get_mut(0..3) {
                s.copy_from_slice(b"TAG");
            }
            if let Some(s) = tail.get_mut(3..13) {
                s.copy_from_slice(b"Test Title");
            }
            if let Some(s) = tail.get_mut(33..44) {
                s.copy_from_slice(b"Test Artist");
            }
            if let Some(s) = tail.get_mut(63..73) {
                s.copy_from_slice(b"Test Album");
            }
            if let Some(s) = tail.get_mut(93..97) {
                s.copy_from_slice(b"2024");
            }
            if let Some(b) = tail.get_mut(125) {
                *b = 0;
            }
            if let Some(b) = tail.get_mut(126) {
                *b = 7; // track 7
            }
        }

        let tags_opt = read_mp3_tags(&mp3);
        assert!(tags_opt.is_some());
        if let Some(tags) = tags_opt {
            assert_eq!(tags.title.as_deref(), Some("Test Title"));
            assert_eq!(tags.artist.as_deref(), Some("Test Artist"));
            assert_eq!(tags.album.as_deref(), Some("Test Album"));
            assert_eq!(tags.year.as_deref(), Some("2024"));
            assert_eq!(tags.track, Some(7));
        }
    }

    #[test]
    fn test_diff_media_tags() {
        let t1 = MediaTags {
            title: Some("Song A".into()),
            artist: Some("Band X".into()),
            track: Some(1),
            ..Default::default()
        };
        let t2 = MediaTags {
            title: Some("Song A".into()),
            artist: Some("Band Y".into()), // Differing!
            track: Some(1),
            album: Some("Album Z".into()), // RightOnly!
            ..Default::default()
        };

        let diff = diff_media_tags(&t1, &t2);
        assert!(!diff.identical);
        assert!(diff
            .matching
            .iter()
            .any(|(f, v)| f == "Title" && v == "Song A"));
        assert!(diff
            .differing
            .iter()
            .any(|(f, l, r)| f == "Artist" && l == "Band X" && r == "Band Y"));
        assert!(diff
            .right_only
            .iter()
            .any(|(f, v)| f == "Album" && v == "Album Z"));
    }

    #[test]
    fn test_hostile_truncated_tags() {
        let truncated = b"fLaC\x84\x00\x10\x00\x01\x02";
        let res = read_flac_tags(truncated);
        assert!(res.is_some()); // Should not panic!
    }

    #[test]
    fn test_synthetic_aac_tags() {
        // Construct an ADTS header:
        // byte 0: 0xFF
        // byte 1: 0xF1 (syncword 0xF, ID 0=MPEG-4, layer 0, protection absent 1)
        // byte 2: 0x50 (profile 1=LC (01), freq_idx 4=44100 (0100), private 0, ch_high 0)
        // byte 3: 0x80 (ch_low 2 = stereo (10), copy 0, home 0, ...)
        let adts = vec![0xFF, 0xF1, 0x50, 0x80, 0x00, 0x00, 0x00];
        let tags_opt = read_aac_tags(&adts);
        assert!(tags_opt.is_some());
        if let Some(tags) = tags_opt {
            assert_eq!(tags.format, MediaFormat::Aac);
            assert!(tags
                .extra_tags
                .iter()
                .any(|(k, v)| k == "Codec" && v == "MPEG-4 AAC"));
            assert!(tags
                .extra_tags
                .iter()
                .any(|(k, v)| k == "Sample Rate" && v == "44100 Hz"));
            assert!(tags
                .extra_tags
                .iter()
                .any(|(k, v)| k == "Channels" && v == "2"));
        }
    }

    fn vorbis_block(comments: &[&[u8]]) -> Vec<u8> {
        let mut block = Vec::new();
        let vendor = b"test vendor";
        block.extend_from_slice(&(vendor.len() as u32).to_le_bytes());
        block.extend_from_slice(vendor);
        block.extend_from_slice(&(comments.len() as u32).to_le_bytes());
        for comment in comments {
            block.extend_from_slice(&(comment.len() as u32).to_le_bytes());
            block.extend_from_slice(comment);
        }
        block
    }

    fn flac_with_comment_block(block: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"fLaC");
        bytes.push(0x84);
        let len = block.len();
        bytes.push(((len >> 16) & 0xFF) as u8);
        bytes.push(((len >> 8) & 0xFF) as u8);
        bytes.push((len & 0xFF) as u8);
        bytes.extend_from_slice(block);
        bytes
    }

    #[test]
    fn test_vorbis_slash_track_and_disc_totals() {
        let block = vorbis_block(&[b"TITLE=Song 9", b"TRACKNUMBER=3/12", b"DISCNUMBER=2/3"]);
        let tags = read_flac_tags(&flac_with_comment_block(&block));
        if let Some(tags) = tags {
            assert_eq!(tags.title.as_deref(), Some("Song 9"));
            assert_eq!(tags.track, Some(3));
            assert_eq!(tags.track_total, Some(12));
            assert_eq!(tags.disc, Some(2));
            assert_eq!(tags.disc_total, Some(3));
        } else {
            assert_eq!(block.len(), 0, "vorbis slash comments must parse");
        }
    }

    #[test]
    fn test_id3_tpos_disc_total() {
        // Minimal ID3v2.3 tag with one TPOS "2/3" frame.
        let mut tag = Vec::new();
        tag.extend_from_slice(b"ID3");
        tag.extend_from_slice(&[3u8, 0u8, 0u8]);
        let frame_body = [0u8, b'2', b'/', b'3'];
        let frame_len = 4 + 4 + 2 + frame_body.len();
        let total = frame_len as u8;
        tag.extend_from_slice(&[0u8, 0u8, 0u8, total]);
        tag.extend_from_slice(b"TPOS");
        tag.extend_from_slice(&(frame_body.len() as u32).to_be_bytes());
        tag.extend_from_slice(&[0u8, 0u8]);
        tag.extend_from_slice(&frame_body);
        let tags = read_mp3_tags(&tag);
        if let Some(tags) = tags {
            assert_eq!(tags.disc, Some(2));
            assert_eq!(tags.disc_total, Some(3));
        } else {
            assert_eq!(tag.len(), 0, "TPOS frame must parse");
        }
    }

    fn mp4_atom(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut atom = Vec::new();
        let size = payload.len().saturating_add(8) as u32;
        atom.extend_from_slice(&size.to_be_bytes());
        atom.extend_from_slice(kind);
        atom.extend_from_slice(payload);
        atom
    }

    fn mp4_data_atom(value: &[u8]) -> Vec<u8> {
        let mut inner = Vec::new();
        inner.extend_from_slice(&[0u8; 8]);
        inner.extend_from_slice(value);
        let mut atom = Vec::new();
        let size = inner.len().saturating_add(8) as u32;
        atom.extend_from_slice(&size.to_be_bytes());
        atom.extend_from_slice(b"data");
        atom.extend_from_slice(&inner);
        atom
    }

    #[test]
    fn test_mp4_disk_and_track_totals() {
        // Apple number pairs: reserved(2), number(2), total(2), reserved(2).
        let trkn_value = [0u8, 0u8, 0u8, 5u8, 0u8, 9u8, 0u8, 0u8];
        let disk_value = [0u8, 0u8, 0u8, 2u8, 0u8, 3u8, 0u8, 0u8];
        let mut ilst_payload = Vec::new();
        ilst_payload.extend_from_slice(&mp4_atom(b"trkn", &mp4_data_atom(&trkn_value)));
        ilst_payload.extend_from_slice(&mp4_atom(b"disk", &mp4_data_atom(&disk_value)));
        ilst_payload.extend_from_slice(&mp4_atom(b"\xa9nam", &mp4_data_atom(b"Song M")));
        let ilst = mp4_atom(b"ilst", &ilst_payload);
        let mut meta_payload = vec![0u8, 0u8, 0u8, 0u8];
        meta_payload.extend_from_slice(&ilst);
        let meta = mp4_atom(b"meta", &meta_payload);
        let udta = mp4_atom(b"udta", &meta);
        let moov = mp4_atom(b"moov", &udta);
        let tags = read_mp4_tags(&moov);
        if let Some(tags) = tags {
            assert_eq!(tags.track, Some(5));
            assert_eq!(tags.track_total, Some(9));
            assert_eq!(tags.disc, Some(2));
            assert_eq!(tags.disc_total, Some(3));
            assert_eq!(tags.title.as_deref(), Some("Song M"));
            assert_eq!(tags.format, MediaFormat::Mp4);
        } else {
            assert_eq!(moov.len(), 0, "mp4 disk/trkn atoms must parse");
        }
        // The generic sniffer reaches the same tags through the fallback chain.
        if let Some(sniffed) = read_media_tags(&moov) {
            assert_eq!(sniffed.format, MediaFormat::Mp4);
            assert_eq!(sniffed.disc_total, Some(3));
        } else {
            assert_eq!(moov.len(), 0, "sniffer must reach mp4 tags");
        }
    }

    #[test]
    fn test_diff_covers_disc_total() {
        let left = MediaTags {
            disc: Some(2),
            disc_total: Some(3),
            ..Default::default()
        };
        let right = MediaTags {
            disc: Some(2),
            disc_total: Some(4),
            ..Default::default()
        };
        let diff = diff_media_tags(&left, &right);
        assert!(!diff.identical);
        assert!(diff
            .differing
            .iter()
            .any(|(f, l, r)| f == "Disc Total" && l == "3" && r == "4"));
        let same = diff_media_tags(&left, &left);
        assert!(same
            .matching
            .iter()
            .any(|(f, v)| f == "Disc Total" && v == "3"));
    }
}

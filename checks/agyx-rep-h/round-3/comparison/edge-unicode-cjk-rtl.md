# Round 3 Edge Fixture: Multilingual, Unicode, CJK, RTL, and Emoji Rendering

Beyond Compare operates across global repos containing international characters:
- CJK (Chinese, Japanese, Korean): `你好，世界！`, `こんにちは世界`
- RTL (Arabic, Hebrew): `مرحبا بالعالم`, `שלום עולם`
- Emojis & Symbols: `🚀 Feature v2.0`, `🔥 Warning`, `🎉 Release`
- Combining marks and quotes: `«Guillemets»`, `„German“`, `‘curly’`

## Fixture Details
- Title, subtitle, diff line content, and child anchor titles contain international characters.

## Expected Outcomes
- UTF-8 valid byte sequences preserved across all output formats.
- HTML and XML escape special characters without corrupting multibyte UTF-8 codepoints.
- Plain Text and Patch render UTF-8 characters cleanly.

## Lock Tests
- `snapshot_tests::test_edge_unicode_cjk_emoji_rtl_rendering`

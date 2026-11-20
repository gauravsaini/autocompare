# Round 1 Edge Fixture: CRLF and Trailing Whitespace Preservation

Beyond Compare parity demands that diff reports accurately handle and preserve carriage return line feeds (`\r\n`) and trailing spaces/tabs without mangling lines, breaking HTML table tags, corrupting XML elements, or losing patch alignment.

## Fixture Details
- Line 1: `fn greet() {\r\n` (CRLF line ending in old and new)
- Line 2 (Delete): `\tlet msg = "hello old";   \r\n` (leading tab, trailing spaces)
- Line 3 (Insert): `\tlet msg = "hello new"; \r\n` (leading tab, different trailing spaces)
- Line 4 (Modify): `    println!("{}", msg);\r\n` (spaces indentation)
- Line 5: `}\r\n`

## Expected Outcomes
- HTML: `pre` block preserves exact whitespace indentation, tabs, and content without unescaped tags.
- Plain Text: line indentation and content preserved.
- XML: `escape_xml` preserves exact string inside `<line>` node.
- Patch: unified format outputs `+\tlet msg...` and `-\tlet msg...` correctly.

## Lock Tests
- `snapshot_tests::test_edge_crlf_and_whitespace_preservation`

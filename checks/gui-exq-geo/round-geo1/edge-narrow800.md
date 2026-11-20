# Edge fixture: narrow-window path-row fit (GEO1)

Live window 800x632 (cr round-6 frame) -> central avail = 800 - 2*12 = 776.
Row chrome (Gap 3): 3 buttons x 120 + handle 8 + 5 gaps x 4 = 388.
Triple chrome: 4 buttons x 120 + 6 gaps x 4 = 504.

| avail | fn | expected |
|---|---|---|
| 776 | split_field_widths(776, 0.50) | (194.0, 194.0), sum + 388 = 776 exact fit |
| 800 | split_field_widths(800, 0.50) | (206.0, 206.0), sum + 388 = 800 exact fit |
| 800 | split_field_widths(800, 0.80) | (232.0, 180.0), clamp floor holds |
| 800 | split_field_widths(800, 0.00) | (180.0, 232.0) = split(800, 0.20) |
| 0 | split_field_widths(0, 0.50) | (180.0, 180.0), 2 x MIN_HALF floor |
| 900 | third_field_width(900) | 132.0, 3 x 132 + 504 = 900 exact fit |
| 0 | third_field_width(0) | 130.0, MIN_THIRD floor |
| 960 | strip 3 pills | 3 x (120 + 24 + 4) = 444 < 960, no scroll |
| 960 | strip 8 pills | 8 x 148 = 1184 > 960, scroll (threshold holds) |

Unit locks: `views::tests::path_row_chrome_matches_paint_widgets`,
`half_fields_share_equal_width`, `third_fields_share_equal_width`,
`split_field_widths_follow_fraction`, `session_strip_overflows_at_eight_tabs`.

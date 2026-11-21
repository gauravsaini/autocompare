# Kittest render — exq-table63 round-1

No table paint wiring changed (headless token refs only); kittest proves
no render regression via the shared real-egui harness.

- `cargo test -p autocompare-gui --test real_egui_shot`: 2 passed
  (real_egui_text_compare_shot, real_egui_text_compare_shot_light).
- Exquisite harness: RESULT PASS, after.png 10946 bytes,
  md5 3ea634d160afca6fc02567487e4e8466 — byte-identical pre/post
  (baseline 10946B same md5), hit behavior unchanged.

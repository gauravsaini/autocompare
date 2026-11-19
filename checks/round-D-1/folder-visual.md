# Round D-1 Folder visual (VIS-FOLDER)

- `after.png` (text fixture): 9914 bytes, PNG magic ok, RESULT: PASS
  (run-visual.sh, untouched).
- `after-folder.png` (folder fixture, `--screenshot` dispatcher jab naam
  me `folder` ho): 10300 bytes, PNG magic ok, distinct 256 (non-blank ok).
  Content: Folder tab highlight, 4 filter checkboxes line, honest counts
  line (`count_folder_rows`), glyph tree (`folder_tree_filtered`, src
  expanded), sync preview panel (Update Both/Mirror labels, preview
  summary, consent hint, Apply button), status bar.
- Console parity (`run-round.sh`): RESULT: PASS (GUI-only change, engine
  untouched).

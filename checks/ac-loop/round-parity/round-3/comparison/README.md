# Round 3 Comparison Edge Fixtures: Folder Sync Deletions & Pruning

This directory contains edge fixtures validating folder sync mirror deletions and pruning against Beyond Compare parity:
1. `sync_left/` and `sync_right/`:
   - Left has `keep.txt` and `nested/child.txt`.
   - Right has `keep.txt` and `stale.txt`.
2. Dry-run plan:
   - Identifies `nested/child.txt` as a copy step (`→`).
   - Identifies `stale.txt` as a deletion/pruning step (`×`).
   - Skips identical `keep.txt` (`~`).
3. Apply phase:
   - Safely executes planned operations.
   - Deletes `stale.txt` on the destination.
   - Copies `nested/child.txt` and ensures parent directory creation.
4. `verify.sh`:
   - Automated script executing both dry-run and apply with `--prune`, confirming target state matches source.

# REPORT — Wave BI-MERGESYNC (merge-view + sync-view geometry parity AUDIT — research only)

Scope: Beyond Compare ke merge-view (3-pane / take-row geometry) aur sync-view
(preview panel / mode / consent / apply-gate) ka humare paint se headless + paint
+ fixture antar ka audit. RESEARCH ONLY — `crates/**` me ZERO edit, koi test bhi
edit nahi chalaya (sirf read-only grep + harness run jo `round-1/` me likhta hai).
Clean-room: BC bahar se observe, apne shabdon me. Colors FROZEN (koi hue baat nahi).

## Method (sirf read)

- Paint blocks: `crates/gui/src/views.rs` merge block (~1505-1830, 1900-1960),
  sync block (~4500-4800), checklist block (~5460-5600).
- Model/strings: `crates/gui/src/tree.rs` sync-preview (~589-760, 1757-1760,
  1909-2003) + three-way tree section (~2485-2656); `crates/gui/src/home.rs:820`
  `sync_consent_hint`; `crates/gui/src/app.rs` Merge tab (~5111-5140), Sync tab
  (~5275-5375), FolderMerge tab (~5375-5420).
- Fixtures: `crates/gui/src/screenshot.rs` dispatcher (~82-120), folder fixture
  (~646-910, preview panel ~826-881), merge fixture (~1118-1260).
- Purane REPORTs: `checks/gui-merge-aa/`, `gui-mergepaint-af/`, `gui-mergepick-ai/`,
  `gui-mergeapp-al/`, `gui-foldersync-aq/`, `gui-syncpaint-ar/`,
  `checks/localrun-ax2/REPORT.md`, `checks/localrun-aw2/REPORT.md`.

## BC expectation (apne shabdon me, clean-room)

- Merge-view: base/left/right teen panes ek row me barabar chaudai ke; HAR conflict
  hunk par take-row (left/right/base/both me se chunna) + pending/decided display;
  neeche apply gate (dry-run + output + Apply).
- Sync-view: preview panel (dry-run steps + counts), mode chunna (dono taraf
  update vs mirror), consent gate (bina consent Apply dabane layak nahi), Apply gate.

## Gap spec table (har gap ke liye exact follow-up)

| ID | BC expectation | Humari haalat | Follow-up spec (fn, lines, change, OWNS file) | Rank |
|----|----------------|---------------|-----------------------------------------------|------|
| BI-M1 | Global take-row poore merge par lagta hai | `paint_merge` (`views.rs:1776-1830`) ka global Take row `resolve_take(&model.lines, 0, choice)` se SIRF index 0 resolve karta hai; multi-hunk me baaki hunks untouched rehte hain jabki per-hunk buttons (`paint_merge_three_pane`, `views.rs:1677`) alag temp-store me stage hote hain | `paint_merge` ke global row ko `apply_merge_picks` + App `merge_picks` adopt par rewire karo (ya global row hatao aur per-hunk + apply-all rakho); fn `paint_merge`, `reconcile_merge_picks`; OWNS: `crates/gui/src/views.rs` + `crates/gui/src/app.rs`, `checks/gui-mergeapply-<xx>/` | P0 |
| BI-S1 | Sync badges + selection folder view me dikhte hain | `paint_folder_sync` (`views.rs:4624-4785`) `#[allow(dead_code)]` hai — Folder tab ise call NAHI karta (inline + `paint_folder_dual` paint hota hai); AQ model + AR paint land ke baad bhi live me badge/selection kuch nahi dikhta | Folder tab me rewiring: `paint_folder_sync` ko mode + `FolderSyncSelection` + expanded + pager pass karo; fn `paint_folder_sync`, App Folder tab (`app.rs` Folder branch); OWNS: `crates/gui/src/app.rs`, `checks/gui-syncwire-<xx>/` | P0 |
| BI-M2 | Folder-merge teen taraf ka faisla dikhta hai | Three-way tree model headless-only hai (`tree.rs:2485-2656`: `three_way_tree_rows`, `is_three_way_conflict`, `three_way_tree_summary`); `views.rs` me koi `paint_three_way*` NAHI; FolderMerge tab (`app.rs:~5375`) flat checklist (`paint_folder_merge_checklist`, `views.rs:5536`) paint karta hai, 3-pane nahi | Naya `paint_three_way_tree` (rows + per-conflict Take Left/Base/Right + header me `summary().summary_line()`); fn nayi, model `tree::three_way_*` reuse; OWNS: `crates/gui/src/views.rs` (+ baad me `app.rs` wiring), `checks/gui-mergemulti-<xx>/` | P1 |
| BI-M3 | Merge Apply gate live hai | `paint_merge_apply` (`views.rs:1912-1960`) `#[allow(dead_code)]` hai — Merge tab (`app.rs:~5111`) ise call NAHI karta; dry-run checkbox + output path + Apply button live me kahin nahi | Merge tab me `paint_merge_apply` wiring (`merge_apply_enabled`, `request_merge_apply`/`run_merge_apply` caller); OWNS: `crates/gui/src/app.rs` + `crates/gui/src/views.rs`, `checks/gui-mergeapply-<xx>/` | P1 |
| BI-S2 | Sync mode paint me chuna jata hai | Mode kahin paint NAHI hota: `paint_folder_sync` mode ko param leta hai par koi `radio_value` nahi (picture tab me precedent `views.rs:~5208`); Sync tab (`app.rs:~5275`) me mode choice NAHI, plan left->right only; folder fixture me mode labels static text hain (`screenshot.rs:~839-850`) | Folder/Sync paint me UpdateBoth/Mirror radio + `set_sync_mode` (`tree.rs:1991` via App `set_folder_sync_mode`, `app.rs:2615`) wiring; OWNS: `crates/gui/src/views.rs` + `crates/gui/src/app.rs`, `checks/gui-syncmode-<xx>/` | P1 |
| BI-S3 | Consent gate ek jagah, preview se judvaan | Gate do raaston me banta hai: `can_apply` (`tree.rs:747-748`) + `FolderSyncSession.consented` + mode-switch reset (`tree.rs:1990-2003`) + `sync_consent_hint` (`home.rs:820`) fixture me dikhta hai, par live Folder tab me consent checkbox NAHI (sirf `folder_sync_dry_run`); Sync tab me dry-run + confirm checkbox hai par woh alag checklist path (`copy_right_plan`) hai, folder path (`apply_mirror_copy`) se alag | Dono apply paths ko ek gate par lao (`can_apply_now` + consent hint paint me); fn `can_apply_now`, App Sync/Folder branches; OWNS: `crates/gui/src/app.rs` + `crates/gui/src/views.rs`, `checks/gui-syncgate-<xx>/` | P1 |
| BI-M4 | Fixture take-row geometry prove karta hai | Merge fixture (`screenshot.rs:1118-1260`, const `MERGE_X0=12/X1=325/X2=638/PANE_W=309`, `ROW_H=24`) 3 static panes + status line hai; take-row strip / decision display NAHI — harness PASS is fixture par hai, live take-row prove nahi hota | Fixture me take-row strip + decision line jodo (ya alag merge-take fixture route); OWNS: `crates/gui/src/screenshot.rs`, `checks/gui-mergefix-<xx>/` | P2 |
| BI-S4 | Har tab ka apna fixture hai | Dispatcher (`screenshot.rs:82-120`) me `"sync"` branch NAHI — sync naam fallback text fixture par girta hai (AW2 precedent: `picture.png` md5 == `text.png` kyunki picture branch nahi tha); koi dedicated sync-tab fixture NAHI | `render_fixture_png` me sync branch + `render_sync_fixture_png` (checklist + dry-run + confirm + apply), ya document karo ki FolderMerge ki tarah merge route reuse hoga; OWNS: `crates/gui/src/screenshot.rs`, `checks/gui-syncfix-<xx>/` | P2 |

## Geometry notes (lazim antar, BC vs hum — barabar shabdon me)

- Merge 3-pane: fixture `309*3=927` + 4px gaps (12/325/638) vs live
  `three_pane_widths` (`views.rs:163-172`): `(avail - 2*SECTION_GAP)/3`,
  `SECTION_GAP=8.0`, `MIN_THIRD` floor, leftover right pane me. Dono equal-thirds
  hain par floor/leftover rule fixture me encode NAHI (static 309).
- Per-hunk card `ui.horizontal` + 3 `vertical(set_width)` hai, Grid NAHI —
  card-to-card column alignment content wrap par drift kar sakta hai; BC row
  columns line me rehte hain. (P1 follow-up `paint_merge_three_pane` Grid par.)
- Sync badge column `sync_badge_min_px` (`tree.rs:~3091`: `SYNC_BADGE_CHARS=2` *
  mono advance + pad) — model-token lock hai, fixture me badge column NAHI hai
  (fixture preview panel sirf summary + hint + Apply dikhata hai, per-row badge nahi).
- Typography: mono caption `FOLDER_SIDE_SIZE=12.0` (`tree.rs:934`), `.size` values
  sirf `['14.0']` (harness report) — koi typo antar nahi mila.

## AX2 / AX / AW me merge-sync ka kya hua tha (recap)

- AX harness (`checks/gui-harness-ax/`): 10/10 shared files md5-identical vs AX2 —
  harness sync byte-level reproduce hota hai.
- AW2 ne `merge.png` nikala HI NAHI tha (AW2 REPORT); AX2 ne pehli baar merge
  baseline darj ki (8829 B, 3-pane, 7 rows) — wahi PNG yahan `round-1/merge-ref.png`
  me copy hai.
- AX2 §13 sabse bada gap: fixture live *contract* par hai, live *paint* purana —
  paint adoption baaki. Yeh audit confirm karta hai: AR (syncpaint) + AQ
  (foldersync) land ke baad bhi `paint_folder_sync` dead hai (BI-S1), aur merge
  me global-take sirf index 0 (BI-M1).
- AW2 precedent: dispatcher me `picture` branch missing tha (`picture.png` md5 ==
  `text.png`) — usi class ka bug BI-S4 (sync branch missing).

## Harness evidence (round-1/)

- `bash tools/gui-polish/run-geo-typo.sh checks/gui-mergesync-bi/round-1` → EXIT 0,
  `RESULT: PASS` (`round-1/geo-typo-report.md`): PNG 9695 bytes (min 8192 ok),
  magic ok, distinct 255 (ok); Grid x10, monospace x53, strong x23, small x50,
  `.size` values sirf `['14.0']` — koi typo change nahi (audit me koi edit nahi).
- `round-1/tests.out` + `round-1/screenshot.out` harness-generated.
- `round-1/merge-ref.png` (8829 B, AX2 merge baseline ki copy) + 
  `round-1/folder-sync-ref.png` (10068 B, folder + sync preview panel ki copy):
  dono `file` me `PNG image data, 960x600, RGBA`, magic `89504e470d0a1a0a` ok,
  distinct 256/256.

## Scope / DoD

- `crates/**` me ZERO edit (read-only grep + harness run allowed tha; `git status`
  me `crates/gui/src/views_hex.rs` ka uncommitted diff parallel agent ka hai —
  chhooa nahi, commit me shaamil nahi).
- `checks/opencode-waves/STATUS.md` ko CHHOOAA NAHI (read-only dekha).
- Colors FROZEN — koi hue/token baat nahi.
- DoD: REPORT me spec table + rank, GATES me gate proofs, round-1 me harness PASS
  proof + 2 existing PNG copies (magic ok), commit local only, NO push.

## Files (sirf OWNS)

- `checks/gui-mergesync-bi/REPORT.md` (yeh file)
- `checks/gui-mergesync-bi/GATES.md`
- `checks/gui-mergesync-bi/round-1/` (`after.png`, `geo-typo-report.md`,
  `tests.out`, `screenshot.out`, `merge-ref.png`, `folder-sync-ref.png`)

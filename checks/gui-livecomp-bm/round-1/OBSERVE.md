# OBSERVE notes — live visual compare vs Beyond Compare (round-1)

Date: 2026-11-20. Method: sirf OBSERVE + CAPTURE. Koi repo file modify nahi ki, koi `.rs` edit nahi, koi window close nahi.

## Environment
- Repo: /Users/ektasaini/autocompare, branch main, `git pull --ff-only` = already up to date (commit e38170b).
- Screen: 2880x1800 px / 1440x900 pt, scale exactly 2.0 (`sips` + Finder desktop bounds se verify).
- Beyond Compare: pehle se khula mila (process BCompare, Text Compare session `t_left.txt <--> t_right.txt` active thi — as-found shot: live-bc-asfound.png).
- Humara app: `target/debug/autocompare-gui` (build 09:35, fresh) launch kiya, pid 1059. Test ke baad bhi RUNNING chhoda hai (reuse ke liye). Koi close nahi kiya.

## Fixtures (dono apps me SAME inputs)
- text: tools/parity-harness/fixtures/t_left.txt + t_right.txt (alpha/beta/gamma vs alpha/BETA/gamma — BC me yehi pair pehle se khula tha)
- folder: tools/parity-harness/fixtures/folder/left + right (f.txt both sides, extra.txt right-only)
- table: tools/parity-harness/fixtures/tab_left.csv + tab_right.csv (id,name,role; ops vs lead)

## Automation method (koi code edit nahi)
- Humara app (egui/accesskit) poora AX tree expose karta hai — tabs = checkboxes, buttons/fields by name click/set kiye (`click checkbox "Text"`, `set value of text field 1`, `click button "Compare files"`). Coordinate guessing fail hui thi, AX exact tha.
- BC AX expose nahi karta. Folder+table sessions `open -a "Beyond Compare" <left> <right>` se khole (naye tabs, non-destructive). Tab switch BC ke Window menu > Select Next/Previous Tab se kiya (coordinate click kaam nahi kar raha tha).
- Har shot `screencapture -x` fullscreen + `sips -c` window-crop (window rect osascript se). Zoom crops diff-line region ke hain.

## BC state note (honest)
- BC me 2 naye session tabs khole gaye (folder, table) taaki same views capture ho saken. Koi tab/window close NAHI kiya. Text session bilkul as-found hai.
- Ours app me Text/Folder/Table tabs me fixture paths load kiye (sirf apne test process me).

## Window rects (points, capture time)
- ours: (320,91) 800x632 — teeno shots me same.
- BC: (720,31) 720x869 — teeno shots me same.

## Anomaly (REPORT me P0 hai, yahan sirf observe)
- Ours Folder view me red debug overlay dikha: "First use of ScrollArea ID D92C" — row 1 ke upar paint hua. Ye string humare crates me nahi hai (grep zero) — egui framework ka emitted text hai, hamare ScrollArea usage se trigger hua lagta hai.
- Ours Table view me duplicate control rows dikhe: 2x Find bars, 2x "no key columns" notes, 2x Prev/Next page rows. Header row data-row ki tarah paint hui ("= id name role" + dashed separator).

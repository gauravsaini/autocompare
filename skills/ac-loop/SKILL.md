---
name: ac-loop
description: Unified autocompare loop harness (CLI parity + GUI geo-typo + live BC observe). Use when verifying comparison features, running a check round, or driving fix-and-repeat refinement.
---

# ac-loop (craft-anything Check loop, unlazy gates, ponytail minimal)

Run: `tools/ac-loop/run.sh checks/ac-loop/round-N` -> `checks/ac-loop/round-N/REPORT.md` (`RESULT: PASS` = CLI PASS + GUI PASS).

## Loop (fix-and-repeat, max 6 rounds)
1. Fresh `checks/ac-loop/round-N/` me run karo.
2. Green: move on. Red: failing `.out`/log padho, smallest root-cause diff owning crate me fix karo, same round dir rerun karo (N advance nahi).
3. Har round me `comparison/` me 1+ naya edge fixture + expected output add karo (recurring comparison files).
4. 2 consecutive no-change rounds ya 10 fails: stop, `checks/inbox/<issue>.md` likho.

## Rules
- Clean-room outside observe: BC behaviour apne words me, strings/assets copy nahi.
- MS rust guidelines: forbid unsafe, no unwrap/expect/panic/todo non-test me, headless-first.
- Geometry first (width/height/margin/padding/align/position), typography second (family/weight/line-height/spacing). Colors FROZEN.
- Disjoint OWNS per agent, conventional commit English only, NO PUSH (orchestrator verifies then pushes). Chat Hinglish-Roman only.
- Max 3-4 parallel agents per wave; future leaves poll with exponential backoff (base 30s, x2, max 10min).
- Mix: opencode default model for edits, agyx swarm for parallel fan-out.
- Probe strategy: event-driven + bounded on-demand harness, fixed timer nahi. File watcher trigger + idle/wall-clock deadlines, max 6 rounds.

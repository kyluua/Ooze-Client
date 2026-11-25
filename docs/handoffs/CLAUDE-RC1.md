# CLAUDE RC1 — Ooze base and launcher release candidate

Owner: Claude (Fabric integration, launcher interface, product verification). Counterpart: Sol (`SOL-RC1.md`).

## Status log

Newest last. Each line names a commit or package hash.

- 2026-11-25 STARTED `7dd54d73f5897dcd2900241865e8dbac56d271ff`.
  - Running in a Linux x86_64 cloud container; no Windows machine is attached (the Remote Control folder list is empty).
  - Windows operation is Sol's, with the owner for Microsoft sign-in and Narrator.

## Plan

- **A.** Review Sol's `44c407f` and today's pushes.
- **B.** Launcher interface fixes B1–B6 (B7 only if `API-READY` lands first).
- **C.** Checks, then `UI-READY`.
- **D.** Verify the RC1 package after `PACKAGE-READY`.
- **E.** Fold the Windows checklist results into `CLAUDE-GATE-L.md`.
- **F.** Prism fallback review.
- **G.** Handoff and Gate L decision.

---
status: accepted
---

# Midnight split ends at the next local midnight

A Midnight split used to end the earlier piece at 23:59:59 so both instants fell on the start's Calendar day. That dropped one second on every cut. The earlier piece now ends at the next local 00:00:00 and the next piece starts at that same instant, for a running Interval and for Save in the Entry editor. Duration is end minus start, so focused time matches the range. Calendar day stays the start's date. Entries already ended at 23:59:59 are left as stored until a later start/end correction.

## Considered options

Ending at 23:59:59 kept both clocks on the start's date, and lost the last second.

## Consequences

New pieces and old 23:59:59 pieces coexist. Opening an old piece without changing its times does not move the end. A full middle day runs from 00:00:00 to the next 00:00:00.

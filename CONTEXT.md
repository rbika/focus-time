# Focus Timer

A macOS menu bar timer. Running time is recorded as focused time the user can review in Stats.

**Entry**:
One contiguous stretch of focused time, bounded by a start and an end. It is either **recorded** from a Start or Resume until Pause, Save of a running Interval, or natural completion, or **created** from Stats as a Manual entry.

Accidental recorded taps of 10 seconds or less are not recorded. A Manual entry is a draft until Save; the 10-second skip does not apply to create or to later corrections. After an Entry exists, start and end can be corrected (any duration is allowed as long as end is after start); type cannot. An Entry can be deleted. Resuming always begins a new Entry.

_Avoid: session, log. Do not treat the 10-second skip as an invariant of a persisted Entry._

**Interval**:
The engine's in-flight stretch of running time, from the most recent Start or Resume until the next Pause, Save, Discard, or natural completion. An Interval is not persisted; when it ends it may produce an Entry (subject to the 10-second skip). Only one Interval can exist at a time, and only while the engine status is `running`.

_Avoid: using "interval" and "entry" interchangeably. An Interval is ephemeral; an Entry is persisted._

**Active interval**:
The currently running Interval, if any (`status === running`). Its elapsed time is `intervalElapsedSecs` from the snapshot. Stats can incorporate the active interval into totals and the entries list as a temporary, non-persisted "running" entry. When the Interval ends (pause, save, completion, discard, or midnight split) the active interval vanishes and may be replaced by a recorded Entry.

_Avoid: treating a paused engine as having an active interval — pause finalizes the Interval._

**Discard**:
Ending a running Interval without recording an Entry. The engine returns to Idle.

_Avoid: Delete (that removes a persisted Entry). Discard never touches existing Entries._

**Type**:
Timer, Stopwatch, or Manual. Timer and Stopwatch are copied from the engine when an interval is recorded. Manual is only for entries created from Stats, shown with the square-pen icon.

_Avoid: treating Manual as an engine mode. The menu bar is still only Timer or Stopwatch._

**Entry editor**:
The Stats surface where an Entry's start and end are written. Add entry and opening an existing Entry are the same surface.
_Avoid: add-entry screen, edit screen._

**Manual entry**:
An Entry created from Stats rather than from a finished interval. Not persisted until Save. The entry editor has no Delete action while creating one.

**Focused time**:
The sum of Entry durations, regardless of type (Timer, Stopwatch, or Manual).

**Calendar day**:
The local-timezone midnight-to-midnight date an Entry belongs to, taken from its start time. Dashboard totals and the Entries list both use this. A running Interval that crosses midnight is split: the portion up to midnight is recorded as an Entry for that day, and a new Interval begins at midnight for the new day.

_Avoid: grouping by end time._

**Midnight split**:
When a running Interval spans local midnight, the engine records the pre-midnight portion as an Entry (ended at 23:59:59) and immediately starts a new Interval (started at 00:00:00) so that no single Entry ever straddles two calendar days. The split is transparent to the user — the timer/stopwatch keeps running without interruption.

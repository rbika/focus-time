# Focus Timer

A macOS menu bar timer. Running time is recorded as focused time the user can review in Stats.

**Entry**:
One contiguous stretch of focused time, bounded by a start and an end. A new or corrected Entry belongs to exactly one Calendar day. It is either **recorded** from a Start or Resume until Pause, Save of a running Interval, or natural completion, or **created** from Stats as a Manual entry.

Accidental recorded taps of 10 seconds or less are not recorded. A Manual entry is a draft until Save; the 10-second skip does not apply to create or to later corrections. After an Entry exists, start and end can be corrected (any duration is allowed as long as end is after start); a correction that would cross local midnight is a Midnight split — the existing Entry becomes the earliest piece, later pieces are new, Type is copied onto every piece. Type cannot be changed otherwise. An Entry can be deleted. All Entries can be deleted at once from Statistics. Resuming always begins a new Entry.

_Avoid: session, log. Do not treat the 10-second skip as an invariant of a persisted Entry._

**Interval**:
The engine's in-flight stretch of running time, from the most recent Start or Resume until the next Pause, Save, Discard, or natural completion. An Interval is not persisted; when it ends it may produce an Entry (subject to the 10-second skip). Only one Interval can exist at a time, and only while the engine status is `running`.

_Avoid: using "interval" and "entry" interchangeably. An Interval is ephemeral; an Entry is persisted._

**Active interval**:
The currently running Interval, if any (`status === running`). Its elapsed time is `intervalElapsedSecs` from the snapshot. Stats can incorporate the active interval into totals and the entries list as a temporary, non-persisted "running" entry. When the Interval ends (pause, save, completion, or discard) the active interval vanishes and may be replaced by a recorded Entry. A Midnight split records the earlier piece and the active interval continues from that midnight.

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

**Stats**:
The window where focused time is reviewed: Dashboard totals and the Entries list.
_Avoid: Statistics for this window._

**Period**:
This week, This month, or This year. The Entries list shows only Entries whose Calendar day falls in the selected Period. This year is first the last three calendar months (clipped to 1 Jan), then extended backward three calendar months at a time; earlier years are not listed.
_Avoid: all, all time, range. Dashboard totals are not a Period — they include Today and have no year._

**Statistics**:
The General Settings group whose action is Reset statistics.
_Avoid: using Statistics as the name of the Stats window._

**Reset statistics**:
Deleting every persisted Entry after confirmation. An active Interval is left running. Settings, presets, shortcuts, and the engine are left alone. The action stays available even when there are no Entries.
_Avoid: timer Reset (engine back to Idle). Do not treat this as Discard._

**Calendar day**:
The local-timezone midnight-to-midnight date an Entry belongs to, taken from its start time. A new or corrected Entry starts on that day; its end falls on that day or on the next local midnight. Dashboard totals and the Entries list both use this.

_Avoid: grouping by end time. Do not assume every persisted Entry already obeys the same-day bound. Do not require the end to fall on the start's date._

**Midnight split**:
Cutting a stretch of focused time at each local midnight so every resulting Entry belongs to one Calendar day. The earlier piece ends at the next local 00:00:00; the next piece starts at that same instant. For a running Interval, the engine records the earlier Entry and continues the Interval from that midnight; countdown remaining and stopwatch elapsed are unchanged. For Save in the Entry editor, one start/end range becomes one Entry per Calendar day, silently: the existing Entry becomes the earliest piece, later pieces are new. Pieces are independent Entries. Entries persisted before this rule may still straddle until a later start/end correction; opening one without changing times does not split it. An Entry already ended at 23:59:59 stays that way until a later start/end correction; opening it without changing times does not move the end to the next midnight.

_Avoid: treating a midnight-crossing range as one Entry. Do not end the earlier piece at 23:59:59._

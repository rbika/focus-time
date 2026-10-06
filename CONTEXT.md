# Focus Time

A macOS menu bar timer. Running time is recorded as focused time the user can review in Stats, except Breaks.

**Entry**:
One contiguous stretch of focused time, bounded by a start and an end. A new or corrected Entry belongs to exactly one Calendar day. It is either **recorded** from a Start or Resume until Pause, Save of a running Interval, or natural completion, or **created** from Stats as a Manual entry. Persisted Entries are oldest-first by start, then by end. A new or corrected Entry is placed by start, not by when it was saved.

Accidental recorded taps of 10 seconds or less are not recorded. A Manual entry is a draft until Save; the 10-second skip does not apply to create or to later corrections. After an Entry exists, start and end can be corrected (any duration is allowed as long as end is after start); a correction that would cross local midnight is a Midnight split — the existing Entry becomes the earliest piece, later pieces are new, Type is copied onto every piece. Type cannot be changed otherwise. An Entry can be deleted. All Entries can be deleted at once from Statistics. Resuming always begins a new Entry.

_Avoid: calling an Entry a Session. Log. Do not treat the 10-second skip as an invariant of a persisted Entry. Do not treat last-saved as newest._

**Interval**:
The engine's in-flight stretch of running time, from the most recent Start or Resume until the next Pause, Save, Discard, Skip, or natural completion. An Interval is not persisted; when it ends it may produce an Entry (subject to the 10-second skip). A Break Interval never produces an Entry. Only one Interval can exist at a time, and only while the engine status is `running`.

_Avoid: using "interval" and "entry" interchangeably. An Interval is ephemeral; an Entry is persisted._

**Active interval**:
The currently running Interval, if any (`status === running`). Its elapsed time is `intervalElapsedSecs` from the snapshot. Stats can incorporate the active interval into totals and the entries list as a temporary, non-persisted "running" entry, except when the Interval is a Break. In the Entries list it is pinned to the top of its Calendar day. When the Interval ends (pause, save, completion, discard, or Skip) the active interval vanishes and may be replaced by a recorded Entry. A Midnight split records the earlier piece and the active interval continues from that midnight, except during a Break, which is not recorded.

_Avoid: treating a paused engine as having an active interval — pause finalizes the Interval. Do not insert the running row by start among persisted Entries. Do not prepend it to the whole list._

**Discard**:
Ending a running Interval without recording an Entry. The engine returns to Idle.

_Avoid: Delete (that removes a persisted Entry). Discard never touches existing Entries._

**Type**:
Timer, Stopwatch, Pomodoro, or Manual. Timer, Stopwatch, and Pomodoro are copied from the engine when an Interval is recorded. Pomodoro is only recorded from a Session, never a Break, and is shown with the tomato icon. Manual is only for entries created from Stats, shown with the square-pen icon.

_Avoid: treating Manual as an engine mode. The menu bar is Timer, Stopwatch, or Pomodoro._

**Pomodoro**:
The third engine mode, alongside Timer and Stopwatch. In this mode the engine runs Sessions and Breaks inside a Cycle.

_Avoid: treating Pomodoro as a Timer preset. Do not call a Break's Type Pomodoro — a Break has no Type._

**Session**:
The work phase of a Cycle: a countdown Interval in Pomodoro mode. Completing or saving one may produce an Entry of Type Pomodoro.

_Avoid: using Session for an Entry, a Timer Interval, or a Break._

**Break**:
A rest phase of a Cycle — Short break or Long break. A countdown Interval in Pomodoro mode that never produces an Entry and is not focused time. After every completed Session the next Break is a Short break, unless the Cycle has reached the configured number of Sessions, in which case it is a Long break.

_Avoid: treating a Break as focused time. Do not record a Break as an Entry._

**Cycle**:
The sequence of Sessions toward the next Long break. It returns to Session 1 when the Long break completes or is Skipped, and that Session is Waiting. Cancel also returns to Session 1, but leaves Waiting. Skip of a Short break does not reset the Cycle. The same Cycle is still the Cycle after quit, and after leaving the Pomodoro tab while idle.

_Avoid: treating a Cycle as an Entry or as focused time._

**Waiting**:
The next phase of a Cycle is selected and has not started. Entered after a Session or Break completes (including while quit), after Skip, and after a Long break returns to Session 1. Cancel leaves it; first picking Pomodoro is not Waiting. Survives quit. A Session that completes while quit still records an Entry subject to the usual rules.

_Avoid: Idle for this. Do not treat Cancel's Session 1 as Waiting. Do not treat a relaunch as Cancel._

**Cancel**:
The running-view action that resets the Cycle to Session 1 and leaves Waiting. If a Session Interval is in flight and longer than 10 seconds, Save or Discard first. If a Break Interval is in flight, Discard immediately — no dialog. If already Waiting, only the Cycle is reset.

_Avoid: Skip (keeps the Cycle). Timer Reset._

**Skip**:
Ends a Break without recording an Entry and without resetting the Cycle. Available while a Break Interval is in flight and while Waiting on a Break. The next phase is Waiting — a Short break's following Session, or Session 1 after a Long break. Not available on a Session. Does not auto-start the next Session.

_Avoid: Cancel. Discard as the name of this action. Do not treat Skip as natural completion._

**Auto-start**:
Settings that start the next Break or the next Session when the current phase completes naturally. Does not apply to Skip. When the matching setting is off, that next phase is Waiting.

_Avoid: treating Skip as a finish that should auto-start._

**Entry editor**:
The Stats surface where an Entry's start and end are written. Add entry and opening an existing Entry are the same surface.
_Avoid: add-entry screen, edit screen._

**Manual entry**:
An Entry created from Stats rather than from a finished interval. Not persisted until Save. The entry editor has no Delete action while creating one.

**Focused time**:
The sum of Entry durations, regardless of type (Timer, Stopwatch, Pomodoro, or Manual). Breaks are not Entries and are not included.

**Timer window**:
The window opened and hidden by a tray left-click. It shows Timer or Stats; last view is kept across hide/show. Settings and the Update progress window are separate windows. A Notification click always shows and focuses this window and does not change the view — it never toggles hide, never closes siblings, and never moves focus into them. A leftover banner after quit relaunches the app and opens this window the same way.

_Avoid: main window. Do not treat Stats as its own window. Do not treat a Notification click as Show Timer (⌘1) or as a tray toggle._

**Stats**:
The view in the Timer window where focused time is reviewed: Dashboard totals and the Entries list. The Entries list is newest-first by start, grouped by Calendar day.
_Avoid: Statistics for this view. Do not treat last-saved as newest. Do not treat Stats as a separate window._

**Period**:
This week (the default), This month, or This year. The Entries list shows only Entries whose Calendar day falls in the selected Period; This week is the seven Calendar days from the current Week start. This year is first the last three calendar months (clipped to 1 Jan), then extended backward three calendar months at a time, and earlier years are not listed.
_Avoid: all, all time, range. Dashboard totals are not a Period — they include Today and have no year. Do not treat This week as a Monday–Sunday ISO week._

**Settings**:
The separate preferences window, with tabs General, Timer, Pomodoro, Notifications, Shortcuts, and About. Closing it hides the window; it stays mounted until quit.

_Avoid: Settings view. Do not treat Settings as a view inside the Timer window. Statistics is a group on General, not this window. Stats is the focused-time review surface._

**Update progress window**:
The window shown after Install update from the available sheet. It has four screens: Downloading (progress and Cancel; Cancel stays visible but disabled while the bar finishes filling), Ready to install (Later / Install and restart), Install failed, and Error (OK). The icon is shared; each screen swaps title, body, and a caption | actions row in one column beside it. Downloading puts bytes and Cancel on that row. Ready to install, Install failed, and Error leave the caption empty so actions sit alone — Error's message is the body, like Install failed. All four screens are the same size. Other update statuses do not show this window.

_Avoid: treating the bar-fill hold as its own screen. Do not treat this as the available sheet or the up-to-date sheet. Do not grow the window. Do not put Error's OK on the same row as the message._

**Install failed**:
The Update progress window screen after Install and restart fails. The downloaded update is still ready; Later and Install and restart stay available. The title stays Ready to install.

_Avoid: Error (that is a failed check or download). Ready to restart as a user-facing name._

**Notification**:
The native macOS banner shown when a Timer Interval, Session, or Break completes naturally, if Allow notifications is on. Clicking one — including a leftover banner after quit — opens the Timer window. The banner has no action buttons. Clicks only work when the app runs from a `.app` bundle; under `tauri dev` the banner still shows but a click does nothing.

_Avoid: Completion sound. Do not treat a click as Show Timer (⌘1) or as a tray toggle._

**Completion sound**:
The sound played when a Timer Interval, Session, or Break completes naturally. Chosen in Settings → Notifications. None plays nothing.

_Avoid: alert, notification sound, chime. Not the notification banner._

**Statistics**:
The General Settings group for Week start and Reset statistics.
_Avoid: using Statistics as the name of the Stats view._

**Week start**:
The weekday a week begins on: Sunday or Monday. Chosen in Statistics. Default Monday. Dashboard This Week and Period This week are the seven local Calendar days from that weekday's midnight; changing Week start redefines This week immediately and does not rewrite Entries.
_Avoid: ISO week, locale first weekday, week starts on as a second concept._

**Reset statistics**:
Deleting every persisted Entry after confirmation. An active Interval is left running. Settings, presets, shortcuts, and the engine are left alone. The action stays available even when there are no Entries.
_Avoid: timer Reset (engine back to Idle). Do not treat this as Discard._

**Calendar day**:
The local-timezone midnight-to-midnight date an Entry belongs to, taken from its start time. A new or corrected Entry starts on that day; its end falls on that day or on the next local midnight. Dashboard totals and the Entries list both use this.

_Avoid: grouping by end time. Do not assume every persisted Entry already obeys the same-day bound. Do not require the end to fall on the start's date._

**Midnight split**:
Cutting a stretch of focused time at each local midnight so every resulting Entry belongs to one Calendar day. The earlier piece ends at the next local 00:00:00; the next piece starts at that same instant. For a running Interval, the engine records the earlier Entry and continues the Interval from that midnight; countdown remaining and stopwatch elapsed are unchanged. A Break that crosses midnight is not recorded; the Break Interval continues. For Save in the Entry editor, one start/end range becomes one Entry per Calendar day, silently: the existing Entry becomes the earliest piece, later pieces are new. Pieces are independent Entries. Entries persisted before this rule may still straddle until a later start/end correction; opening one without changing times does not split it. An Entry already ended at 23:59:59 stays that way until a later start/end correction; opening it without changing times does not move the end to the next midnight.

_Avoid: treating a midnight-crossing range as one Entry. Do not end the earlier piece at 23:59:59._

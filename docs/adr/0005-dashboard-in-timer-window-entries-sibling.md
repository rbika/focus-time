# Dashboard lives in the Timer window; Entries is a sibling window

Stats as a swapped view inside the Timer window (Dashboard | Entries tabs, chart/hourglass toggle, animated resize, last-view) is retired. The Timer window is always timer on the left and Dashboard on the right. The Entries list and Entry editor move to a Settings-shaped sibling window, opened from a title-bar icon.

Keeping both surfaces in one window forced a compact list, a view swap, and hiding the timer to review history. Putting Dashboard beside the timer makes focused-time totals visible while an Interval is in flight. Entries needs add/edit/Period/scroll, which fits a separate window like Settings — not a second tab in a tray-sized popup.

**Considered Options**: keep the Stats swap and only restyle it; put the Entries list in the Timer window under Dashboard; make Entries a tab of Settings.

**Consequences**: `resize_main_window` and `MainView` go away. ADR-0002's overlay still applies, but Dashboard overlay runs whenever the Timer window is mounted and Entries overlay runs in the Entries webview. ADR-0004's "last view kept" no longer applies — a Notification click still shows the Timer window, which has only one layout. There is no Show Stats / Show Entries shortcut.

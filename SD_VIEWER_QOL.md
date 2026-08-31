# SD Viewer — QoL Improvements

## High Impact / Low Effort

- **Keyboard navigation on timeline** — Arrow keys step through records, Page Up/Down jump 100 records. Slider alone is clunky for precision scrubbing.
- **Zoom to flight segment** — Click a state segment on the timeline to auto-zoom all charts to that time range. Finding a 2-second burn in a 10-minute log is tedious otherwise.

## Medium Effort

- **Export to CSV** — One-click dump of all series data (timestamps, accel, gyro, GPS, pressure, temp, battery). Useful for post-flight analysis in Excel/MATLAB.
- **Record inspector panel** — Side panel showing all raw fields of the currently selected record. Currently only partial info is visible.
- **Auto-detect interesting moments** — Highlight max altitude, max acceleration, max velocity automatically. Add jump-to buttons for each.
- **Bookmark/annotation markers** — Let the user drop named markers on the timeline during review. Session-only persistence.

## Higher Effort

- **Multi-file compare** — Load two .bin files side-by-side for comparing test flights.

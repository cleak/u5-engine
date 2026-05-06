# u5-engine

Verification harness for the Ultima V cleanroom specs.

This is not a full replacement engine yet. The first executable is a narrow
Lord British castle/throne-room slice that reads the user's local Ultima V data
at runtime and checks the current public specs against real files:

- town-mode scene partitioning;
- per-class `*.DAT`, `*.NPC`, and `*.TLK` joins;
- location floor loading and render-class hashing;
- marker, door, and stair detection;
- schedule waypoint sampling;
- conversation-name lookup; and
- a small class-derived movement/pathfinding smoke test.

The repository does not include game assets. Run it with a local Ultima V
install path:

```powershell
cargo run -- C:\Games\U5-Clean
```

The run writes an aggregate report to
`reports/lb-throne-room-slice.txt`. The report intentionally avoids raw map
dumps, dialogue transcripts, and binary offsets.

"""Regenerates tests/fixtures/sessions/*.json. Run from the repo root.
Events are generated; the expected sessions are derived by hand in the comments.
"""
import json, os
OUT = "tests/fixtures/sessions"
VS, TERM, MAIL, BOOKS = "com.microsoft.VSCode", "com.apple.Terminal", "com.apple.mail", "com.apple.iBooksX"
NAMES = {VS: "Code", TERM: "Terminal", MAIL: "Mail", BOOKS: "Books"}

def T(hm, day="2026-03-02"):
    if len(hm) == 5: hm += ":00"
    return f"{day}T{hm}.000Z"
def mins(hm): h, m = hm.split(":")[:2]; return int(h)*60+int(m)
def hm(total): return f"{total//60:02d}:{total%60:02d}"

def active(t, app): return {"type": "active_application", "timestamp": T(t), "bundleId": app, "applicationName": NAMES[app]}
def switch(t, a, b): return [{"type": "application_switch", "timestamp": T(t), "fromBundleId": a, "toBundleId": b}, active(t, b)]
def idle(t, secs): return {"type": "idle", "timestamp": T(t), "seconds": secs}
def hb(app, start, end):  # every 5 minutes, both ends inclusive
    return [active(hm(m), app) for m in range(mins(start), mins(end)+1, 5)]

def session(start, end, active_m, idle_m, sw, cat, apps):
    return {"id": f"session-{T(start)}", "startedAt": T(start), "endedAt": T(end) if end else None,
            "activeMinutes": active_m, "idleMinutes": idle_m, "contextSwitches": sw,
            "category": cat, "applicationIds": apps, "projectId": None}

def write(name, description, now, events, expected, **extra):
    doc = {"description": description, "now": T(now), **extra, "events": events, "expected": expected}
    with open(f"{OUT}/{name}.json", "w") as f:
        json.dump(doc, f, indent=1)
        f.write("\n")

# A: two switches inside one open session. 09:00 -> now 10:02 = 62 min, no idle.
write("single_session_with_switches",
      "One app, a detour to Terminal and back. Open session; two context switches.",
      "10:02",
      [active("09:00", VS)] + hb(VS, "09:05", "09:25") + switch("09:30", VS, TERM) + switch("09:31", TERM, VS) + hb(VS, "09:36", "10:00"),
      [session("09:00", None, 62.0, 0.0, 2, "Unknown", [VS, TERM])])

# B: 30 min idle (>= 15) at 09:58 ends the session at 09:58 (58 active min); the user
# is back at 10:28, new session open until now=10:40 -> 12 min.
write("long_idle_splits_session",
      "Idle of 30 minutes closes the session at the last input; a new one starts on return.",
      "10:40",
      [active("09:00", VS)] + hb(VS, "09:05", "09:55") + [idle("09:58", 1800)] + hb(VS, "10:33", "10:38"),
      [session("09:00", "09:58", 58.0, 0.0, 0, "Unknown", [VS]),
       session("10:28", None, 12.0, 0.0, 0, "Unknown", [VS])])

# C: 5 min idle (< 15) stays inside: 09:00 -> 09:40 = 40 min, idle 5, active 35.
write("short_idle_stays_in_session",
      "A 5 minute idle period does not split the session; it is counted as idle time.",
      "09:40",
      [active("09:00", VS)] + hb(VS, "09:05", "09:15") + [idle("09:18", 300)] + hb(VS, "09:28", "09:38"),
      [session("09:00", None, 35.0, 5.0, 0, "Unknown", [VS])])

# D: nothing observed 09:10 -> 11:00 (app off). Session 1 ends at its last event, 09:10 (10 min).
# Session 2: 11:00 -> now 11:07 = 7 min.
write("sensor_gap_closes_session",
      "No events for hours (app was off) must not be counted as activity.",
      "11:07",
      [active("09:00", VS)] + hb(VS, "09:05", "09:10") + [active("11:00", VS)] + hb(VS, "11:05", "11:05"),
      [session("09:00", "09:10", 10.0, 0.0, 0, "Unknown", [VS]),
       session("11:00", None, 7.0, 0.0, 0, "Unknown", [VS])])

# E: 09:12-09:22 idle (10 min). At 09:15 Mail steals focus (not the user: not counted).
# 09:30 the user switches to VS Code: the only counted switch. Total 36, idle 10, active 26.
write("switches_during_idle_are_not_counted",
      "A background focus change while the user is away is not a context switch.",
      "09:36",
      [active("09:00", VS)] + hb(VS, "09:05", "09:10") + [idle("09:12", 600)] + switch("09:15", VS, MAIL) + switch("09:30", MAIL, VS) + hb(VS, "09:35", "09:35"),
      [session("09:00", None, 26.0, 10.0, 1, "Unknown", [VS, MAIL])])

# F: VS Code 09:00-09:20 (20) + 09:50-09:55 (5) = 25 min Create; Books 09:20-09:50 = 30 min Learn.
write("category_follows_dominant_time",
      "Category is the one with the most active time, not the first app or the last.",
      "09:55",
      [active("09:00", VS)] + hb(VS, "09:05", "09:15") + switch("09:20", VS, BOOKS) + hb(BOOKS, "09:25", "09:45") + switch("09:50", BOOKS, VS) + hb(VS, "09:55", "09:55"),
      [session("09:00", None, 55.0, 0.0, 2, "Learn", [VS, BOOKS])],
      categories={VS: "Create", BOOKS: "Learn"})

# G: idle in progress since 09:52 and now 10:10 (18 min >= 15): session ended at 09:52 (52 min); no open session.
write("live_idle_beyond_break_closes_session",
      "Idle still in progress for 15+ minutes: the session already ended at the last input.",
      "10:10",
      [active("09:00", VS)] + hb(VS, "09:05", "09:50"),
      [session("09:00", "09:52", 52.0, 0.0, 0, "Unknown", [VS])],
      liveIdleSince=T("09:52"))

# H: idle in progress since 09:52 and now 10:00 (8 min): open; total 60, idle 8, active 52.
write("live_idle_in_progress_is_not_active_time",
      "Idle in progress is subtracted from the open session.",
      "10:00",
      [active("09:00", VS)] + hb(VS, "09:05", "09:50"),
      [session("09:00", None, 52.0, 8.0, 0, "Unknown", [VS])],
      liveIdleSince=T("09:52"))

# I: without any known app there is nothing to attribute time to.
write("idle_without_known_app_makes_no_session",
      "Idle events alone never create a session.",
      "10:00",
      [idle("09:00", 3000)],
      [])

//! Milestone 2 exit criteria: switching and idle are detected as neutral events,
//! disabling a sensor stops collection, and no content is captured.

use chrono::{DateTime, Duration, TimeZone, Utc};
use personal_rhythm_assistant_lib::persistence::repositories::{activity_events, settings};
use personal_rhythm_assistant_lib::persistence::Database;
use personal_rhythm_assistant_lib::privacy::toggles::{self, PrivacyToggles};
use personal_rhythm_assistant_lib::sensors::bus::EventBus;
use personal_rhythm_assistant_lib::sensors::collector::Collector;
use personal_rhythm_assistant_lib::sensors::event::ActivityEvent;
use personal_rhythm_assistant_lib::sensors::probe::{AppInfo, Probe};
use personal_rhythm_assistant_lib::sensors::service::{SensorService, SharedSnapshot};
use personal_rhythm_assistant_lib::sensors::system_state::ActivityState;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

const OWN: &str = "app.personalrhythm.assistant";
const VSCODE: &str = "com.microsoft.VSCode";
const TERMINAL: &str = "com.apple.Terminal";

fn t0() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 3, 2, 9, 0, 0).unwrap()
}
fn at(secs: i64) -> DateTime<Utc> {
    t0() + Duration::seconds(secs)
}
fn app(id: &str) -> Option<AppInfo> {
    Some(AppInfo {
        bundle_id: id.into(),
        name: format!("name of {id}"),
    })
}
fn on() -> PrivacyToggles {
    PrivacyToggles::default()
}

// ---- collector: active app + switching ------------------------------------

#[test]
fn first_observation_emits_only_active_application() {
    let mut c = Collector::with_defaults(vec![]);
    let events = c.tick(at(0), &on(), app(VSCODE), Some(0.0));
    assert_eq!(events.len(), 1);
    assert!(
        matches!(&events[0], ActivityEvent::ActiveApplication { bundle_id, .. } if bundle_id == VSCODE)
    );
}

#[test]
fn staying_in_the_same_app_emits_nothing() {
    let mut c = Collector::with_defaults(vec![]);
    c.tick(at(0), &on(), app(VSCODE), Some(0.0));
    assert!(c.tick(at(2), &on(), app(VSCODE), Some(0.0)).is_empty());
}

#[test]
fn changing_app_emits_a_switch_then_the_new_active_application() {
    let mut c = Collector::with_defaults(vec![]);
    c.tick(at(0), &on(), app(VSCODE), Some(0.0));
    let events = c.tick(at(30), &on(), app(TERMINAL), Some(0.0));
    assert_eq!(events.len(), 2);
    assert_eq!(
        events[0],
        ActivityEvent::ApplicationSwitch {
            timestamp: "2026-03-02T09:00:30.000Z".into(),
            from_bundle_id: VSCODE.into(),
            to_bundle_id: TERMINAL.into(),
        }
    );
    assert!(
        matches!(&events[1], ActivityEvent::ActiveApplication { bundle_id, .. } if bundle_id == TERMINAL)
    );
}

#[test]
fn this_app_is_not_user_activity() {
    let mut c = Collector::with_defaults(vec![OWN.into()]);
    c.tick(at(0), &on(), app(VSCODE), Some(0.0));
    // Glancing at our own window and back is not a context switch.
    assert!(c.tick(at(2), &on(), app(OWN), Some(0.0)).is_empty());
    assert!(c.tick(at(4), &on(), app(VSCODE), Some(0.0)).is_empty());
    // ...and going VSCode -> (own) -> Terminal is one switch, VSCode -> Terminal.
    c.tick(at(6), &on(), app(OWN), Some(0.0));
    let events = c.tick(at(8), &on(), app(TERMINAL), Some(0.0));
    assert!(
        matches!(&events[0], ActivityEvent::ApplicationSwitch { from_bundle_id, .. } if from_bundle_id == VSCODE)
    );
}

#[test]
fn no_frontmost_app_emits_nothing() {
    let mut c = Collector::with_defaults(vec![]);
    assert!(c.tick(at(0), &on(), None, Some(0.0)).is_empty());
}

// ---- collector: idle ------------------------------------------------------

#[test]
fn idle_period_is_reported_once_it_ends_with_exact_start_and_length() {
    let mut c = Collector::with_defaults(vec![]);
    // Last input at t0. Threshold is 120 s.
    assert!(c.tick(at(0), &on(), None, Some(0.0)).is_empty());
    assert!(c.tick(at(60), &on(), None, Some(60.0)).is_empty());
    assert_eq!(c.snapshot().state, ActivityState::Active, "below threshold");

    assert!(
        c.tick(at(130), &on(), None, Some(130.0)).is_empty(),
        "nothing emitted while idle"
    );
    assert_eq!(c.snapshot().state, ActivityState::Idle);
    assert!(c.tick(at(200), &on(), None, Some(200.0)).is_empty());

    // New input at t0+228 (idle counter reads 2 s at t0+230).
    let events = c.tick(at(230), &on(), None, Some(2.0));
    assert_eq!(
        events,
        vec![ActivityEvent::Idle {
            timestamp: "2026-03-02T09:00:00.000Z".into(),
            seconds: 228
        }]
    );
    assert_eq!(c.snapshot().state, ActivityState::Active);
    assert!(
        c.tick(at(232), &on(), None, Some(4.0)).is_empty(),
        "reported once"
    );
}

#[test]
fn a_short_pause_is_not_idle() {
    let mut c = Collector::with_defaults(vec![]);
    c.tick(at(0), &on(), None, Some(0.0));
    c.tick(at(100), &on(), None, Some(100.0));
    assert!(c.tick(at(102), &on(), None, Some(1.0)).is_empty());
}

#[test]
fn state_is_unknown_when_idle_detection_is_off() {
    let mut c = Collector::with_defaults(vec![]);
    let toggles = PrivacyToggles {
        idle_detection: false,
        ..on()
    };
    c.tick(at(0), &toggles, None, None);
    assert_eq!(c.snapshot().state, ActivityState::Unknown);
}

// ---- collector: toggles ---------------------------------------------------

#[test]
fn disabling_active_application_stops_app_events_and_forgets_the_previous_app() {
    let mut c = Collector::with_defaults(vec![]);
    c.tick(at(0), &on(), app(VSCODE), Some(0.0));

    let off = PrivacyToggles {
        active_application: false,
        ..on()
    };
    assert!(c.tick(at(2), &off, None, Some(0.0)).is_empty());
    assert_eq!(
        c.snapshot().frontmost_application,
        None,
        "nothing about the app is retained"
    );

    // Re-enabled in another app: a fresh start, not a fabricated switch from VSCode.
    let events = c.tick(at(60), &on(), app(TERMINAL), Some(0.0));
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0], ActivityEvent::ActiveApplication { .. }));
}

#[test]
fn disabling_idle_detection_discards_an_idle_period_in_progress() {
    let mut c = Collector::with_defaults(vec![]);
    c.tick(at(0), &on(), None, Some(0.0));
    c.tick(at(130), &on(), None, Some(130.0));
    assert_eq!(c.snapshot().state, ActivityState::Idle);

    let off = PrivacyToggles {
        idle_detection: false,
        ..on()
    };
    c.tick(at(140), &off, None, None);
    let events = c.tick(at(150), &on(), None, Some(1.0));
    assert!(
        events.is_empty(),
        "no idle event may be produced from data seen while disabled"
    );
}

// ---- privacy toggles ------------------------------------------------------

#[test]
fn toggle_defaults_match_the_spec() {
    let db = Database::open_in_memory().unwrap();
    let t = db.with_conn(toggles::load).unwrap();
    assert!(t.active_application && t.active_time && t.idle_detection);
    assert!(!t.window_title && !t.keyboard_mouse_rhythm && !t.calendar && !t.cloud_processing);
}

#[test]
fn placeholder_toggles_cannot_be_switched_on() {
    let db = Database::open_in_memory().unwrap();
    let everything_on = PrivacyToggles {
        active_application: true,
        active_time: true,
        idle_detection: true,
        window_title: true,
        keyboard_mouse_rhythm: true,
        calendar: true,
        cloud_processing: true,
    };
    let stored = db.with_conn(|c| toggles::save(c, everything_on)).unwrap();
    assert!(
        !stored.window_title
            && !stored.keyboard_mouse_rhythm
            && !stored.calendar
            && !stored.cloud_processing
    );
    assert_eq!(db.with_conn(toggles::load).unwrap(), stored);
}

// ---- event bus ------------------------------------------------------------

#[test]
fn bus_fans_out_and_drops_dead_subscribers() {
    let bus = EventBus::new();
    let a = bus.subscribe();
    let b = bus.subscribe();
    let e = ActivityEvent::Idle {
        timestamp: "2026-03-02T09:00:00.000Z".into(),
        seconds: 1,
    };
    bus.publish(&e);
    assert_eq!(a.try_recv().unwrap(), e);
    assert_eq!(b.try_recv().unwrap(), e);

    drop(b);
    bus.publish(&e);
    assert_eq!(bus.subscriber_count(), 1);
}

// ---- service: end to end with a scripted OS -------------------------------

#[derive(Clone, Default)]
struct Script {
    app: Arc<Mutex<Option<AppInfo>>>,
    idle: Arc<Mutex<f64>>,
    app_probes: Arc<AtomicUsize>,
    idle_probes: Arc<AtomicUsize>,
}
impl Probe for Script {
    fn frontmost_app(&self) -> Option<AppInfo> {
        self.app_probes.fetch_add(1, Ordering::SeqCst);
        self.app.lock().unwrap().clone()
    }
    fn idle_seconds(&self) -> f64 {
        self.idle_probes.fetch_add(1, Ordering::SeqCst);
        *self.idle.lock().unwrap()
    }
}

struct Rig {
    service: SensorService,
    db: Arc<Database>,
    script: Script,
    bus: Arc<EventBus>,
    snapshot: SharedSnapshot,
}
fn rig() -> Rig {
    let db = Arc::new(Database::open_in_memory().unwrap());
    let script = Script::default();
    let bus = Arc::new(EventBus::new());
    let snapshot = SharedSnapshot::default();
    let service = SensorService::new(
        Box::new(script.clone()),
        Collector::with_defaults(vec![OWN.into()]),
        db.clone(),
        bus.clone(),
        snapshot.clone(),
    );
    Rig {
        service,
        db,
        script,
        bus,
        snapshot,
    }
}
fn stored(db: &Database) -> u64 {
    db.with_conn(activity_events::count).unwrap()
}

#[test]
fn service_persists_and_publishes_events_from_the_probe() {
    let mut r = rig();
    let rx = r.bus.subscribe();

    *r.script.app.lock().unwrap() = app(VSCODE);
    assert_eq!(r.service.tick(at(0)).unwrap(), 1);
    *r.script.app.lock().unwrap() = app(TERMINAL);
    assert_eq!(r.service.tick(at(30)).unwrap(), 2);

    assert_eq!(stored(&r.db), 3);
    assert_eq!(rx.try_iter().count(), 3);
    assert_eq!(
        r.snapshot.lock().unwrap().frontmost_application.as_deref(),
        Some("name of com.apple.Terminal")
    );
}

#[test]
fn disabling_a_sensor_stops_collection_and_stops_probing_the_os() {
    let mut r = rig();
    *r.script.app.lock().unwrap() = app(VSCODE);
    r.service.tick(at(0)).unwrap();
    assert_eq!(stored(&r.db), 1);

    // Switch both sensors off in the database, as the settings screen would.
    r.db.with_conn(|c| toggles::save(c, PrivacyToggles::all_off()))
        .unwrap();
    let (app_probes, idle_probes) = (
        r.script.app_probes.load(Ordering::SeqCst),
        r.script.idle_probes.load(Ordering::SeqCst),
    );

    *r.script.app.lock().unwrap() = app(TERMINAL);
    *r.script.idle.lock().unwrap() = 500.0;
    assert_eq!(r.service.tick(at(30)).unwrap(), 0);
    assert_eq!(r.service.tick(at(600)).unwrap(), 0);

    assert_eq!(stored(&r.db), 1, "nothing recorded while off");
    assert_eq!(
        r.script.app_probes.load(Ordering::SeqCst),
        app_probes,
        "OS not queried for apps"
    );
    assert_eq!(
        r.script.idle_probes.load(Ordering::SeqCst),
        idle_probes,
        "OS not queried for idle"
    );
}

#[test]
fn each_sensor_can_be_disabled_independently() {
    let mut r = rig();
    r.db.with_conn(|c| {
        toggles::save(
            c,
            PrivacyToggles {
                idle_detection: false,
                ..on()
            },
        )
    })
    .unwrap();
    *r.script.app.lock().unwrap() = app(VSCODE);
    r.service.tick(at(0)).unwrap();
    assert_eq!(r.script.idle_probes.load(Ordering::SeqCst), 0);
    assert_eq!(r.script.app_probes.load(Ordering::SeqCst), 1);
}

#[test]
fn unreadable_toggles_fail_closed() {
    let mut r = rig();
    r.db.with_conn(|c| settings::set_json(c, "privacy_toggles", &"not a toggles object"))
        .unwrap();
    *r.script.app.lock().unwrap() = app(VSCODE);
    assert_eq!(r.service.tick(at(0)).unwrap(), 0);
    assert_eq!(stored(&r.db), 0);
    assert_eq!(r.script.app_probes.load(Ordering::SeqCst), 0);
}

// ---- no content -----------------------------------------------------------

#[test]
fn events_carry_only_neutral_identifiers_and_numbers() {
    let events = [
        ActivityEvent::ActiveApplication {
            timestamp: "t".into(),
            bundle_id: "b".into(),
            application_name: "n".into(),
        },
        ActivityEvent::Idle {
            timestamp: "t".into(),
            seconds: 1,
        },
        ActivityEvent::ApplicationSwitch {
            timestamp: "t".into(),
            from_bundle_id: "a".into(),
            to_bundle_id: "b".into(),
        },
    ];
    let allowed = [
        "type",
        "timestamp",
        "bundleId",
        "applicationName",
        "seconds",
        "fromBundleId",
        "toBundleId",
    ];
    for e in &events {
        let json = serde_json::to_value(e).unwrap();
        for key in json.as_object().unwrap().keys() {
            assert!(
                allowed.contains(&key.as_str()),
                "unexpected field {key} — content must never be captured"
            );
        }
    }
}

// ---- macOS adapter smoke test ----------------------------------------------

#[cfg(target_os = "macos")]
#[test]
fn macos_idle_probe_returns_a_sane_elapsed_time() {
    use personal_rhythm_assistant_lib::platform::macos::MacProbe;
    let secs = MacProbe.idle_seconds();
    assert!(
        secs.is_finite() && secs >= 0.0,
        "unexpected idle seconds: {secs}"
    );
}

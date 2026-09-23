//! Milestone 0: proves the selected SQLite dependency (rusqlite, bundled)
//! links and works offline. Replaced by real repository tests in Milestone 1.

use rusqlite::Connection;

#[test]
fn bundled_sqlite_opens_in_memory_and_round_trips() {
    let conn = Connection::open_in_memory().expect("open in-memory database");
    conn.execute_batch("CREATE TABLE t (v TEXT NOT NULL); INSERT INTO t (v) VALUES ('ok');")
        .expect("create and insert");
    let v: String = conn
        .query_row("SELECT v FROM t", [], |row| row.get(0))
        .expect("select");
    assert_eq!(v, "ok");
}

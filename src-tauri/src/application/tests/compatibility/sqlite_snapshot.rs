use super::*;
use crate::model::{ExternalTimestampStatus, ExternalTimestampSummary};
use rusqlite::{types::ValueRef, Connection};
use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct LogicalDatabaseSnapshot {
    schema_objects: Vec<LogicalSchemaObjectSnapshot>,
    tables: BTreeMap<String, LogicalTableSnapshot>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct LogicalSchemaObjectSnapshot {
    object_type: String,
    name: String,
    table_name: String,
    sql: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct LogicalTableSnapshot {
    create_sql: String,
    columns: Vec<LogicalColumnSnapshot>,
    rows: Vec<Vec<LogicalValue>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct LogicalColumnSnapshot {
    name: String,
    declared_type: String,
    not_null: bool,
    default_value: Option<String>,
    primary_key_position: i64,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
enum LogicalValue {
    Null,
    Integer(i64),
    RealBits(u64),
    Text(Vec<u8>),
    Json(String),
    Blob(Vec<u8>),
}

pub(super) fn logical_database_snapshot(app: &WorkspaceApp) -> LogicalDatabaseSnapshot {
    let connection = app
        .persistence
        .open()
        .expect("open SQLite for normalized logical snapshot");
    let schema_objects = user_schema_objects(&connection);
    let definitions = schema_objects
        .iter()
        .filter(|object| object.object_type == "table")
        .map(|object| {
            (
                object.name.clone(),
                object.sql.clone().expect("user-defined table CREATE SQL"),
            )
        });
    let tables = definitions
        .map(|(name, create_sql)| {
            let table = logical_table_snapshot(&connection, &name, create_sql);
            (name, table)
        })
        .collect();
    LogicalDatabaseSnapshot {
        schema_objects,
        tables,
    }
}

pub(super) fn logical_database_digest(snapshot: &LogicalDatabaseSnapshot) -> String {
    sha256_bytes(
        &serde_json::to_vec(snapshot).expect("serialize normalized logical SQLite snapshot"),
    )
}

pub(super) fn assert_expected_database_changes(
    before: &LogicalDatabaseSnapshot,
    after: &LogicalDatabaseSnapshot,
    track_before: &TrackRecord,
    finalized_track: &TrackRecord,
    controlled_profile: &Profile,
) {
    assert_eq!(
        before.schema_objects, after.schema_objects,
        "user-defined SQLite schema-object inventory changed"
    );
    assert_eq!(
        before.tables.keys().collect::<Vec<_>>(),
        after.tables.keys().collect::<Vec<_>>(),
        "user-defined SQLite table inventory changed"
    );
    for (name, before_table) in &before.tables {
        let after_table = after.tables.get(name).expect("snapshot table after pass");
        assert_eq!(
            before_table.create_sql, after_table.create_sql,
            "SQLite CREATE TABLE contract changed for {name}"
        );
        assert_eq!(
            before_table.columns, after_table.columns,
            "SQLite column contract changed for {name}"
        );
        match name.as_str() {
            "profile" => assert_profile_change(before_table, after_table, controlled_profile),
            "tracks" => {
                assert_track_change(before_table, after_table, track_before, finalized_track)
            }
            "timestamp_attachment_status" => {
                assert_finalization_summary_change(before_table, after_table, finalized_track)
            }
            _ => assert_eq!(
                before_table.rows, after_table.rows,
                "unexpected logical SQLite row change in table {name}"
            ),
        }
    }
}

fn user_schema_objects(connection: &Connection) -> Vec<LogicalSchemaObjectSnapshot> {
    let mut statement = connection
        .prepare(
            "SELECT type, name, tbl_name, sql FROM sqlite_schema
             WHERE name NOT GLOB 'sqlite_*'
             ORDER BY type, name, tbl_name",
        )
        .expect("prepare user-defined SQLite schema-object inventory");
    statement
        .query_map([], |row| {
            Ok(LogicalSchemaObjectSnapshot {
                object_type: row.get(0)?,
                name: row.get(1)?,
                table_name: row.get(2)?,
                sql: row.get(3)?,
            })
        })
        .expect("query user-defined SQLite schema-object inventory")
        .collect::<rusqlite::Result<Vec<_>>>()
        .expect("read user-defined SQLite schema-object inventory")
}

#[test]
fn user_schema_inventory_excludes_only_the_reserved_sqlite_prefix() {
    let connection = Connection::open_in_memory().expect("open schema-filter fixture");
    connection
        .execute_batch(
            "CREATE TABLE sqliteX_user_defined (value TEXT NOT NULL);
             CREATE TABLE ordinary_user_defined (id INTEGER PRIMARY KEY AUTOINCREMENT);",
        )
        .expect("create user and SQLite-internal schema objects");

    let names = user_schema_objects(&connection)
        .into_iter()
        .map(|object| object.name)
        .collect::<Vec<_>>();
    assert!(names.iter().any(|name| name == "sqliteX_user_defined"));
    assert!(names.iter().any(|name| name == "ordinary_user_defined"));
    assert!(names.iter().all(|name| !name.starts_with("sqlite_")));
}

fn logical_table_snapshot(
    connection: &Connection,
    table_name: &str,
    create_sql: String,
) -> LogicalTableSnapshot {
    let columns = table_columns(connection, table_name);
    let query = format!("SELECT * FROM {}", quote_identifier(table_name));
    let mut statement = connection
        .prepare(&query)
        .expect("prepare normalized SQLite table rows");
    let mut query_rows = statement.query([]).expect("query normalized SQLite rows");
    let mut rows = Vec::new();
    while let Some(row) = query_rows.next().expect("advance normalized SQLite rows") {
        let values = columns
            .iter()
            .enumerate()
            .map(|(index, column)| {
                logical_value(
                    &column.name,
                    row.get_ref(index).expect("read normalized SQLite value"),
                )
            })
            .collect();
        rows.push(values);
    }
    rows.sort();
    LogicalTableSnapshot {
        create_sql,
        columns,
        rows,
    }
}

fn table_columns(connection: &Connection, table_name: &str) -> Vec<LogicalColumnSnapshot> {
    let query = format!("PRAGMA table_info({})", quote_identifier(table_name));
    let mut statement = connection
        .prepare(&query)
        .expect("prepare normalized SQLite column inventory");
    statement
        .query_map([], |row| {
            Ok(LogicalColumnSnapshot {
                name: row.get(1)?,
                declared_type: row.get(2)?,
                not_null: row.get::<_, i64>(3)? != 0,
                default_value: row.get(4)?,
                primary_key_position: row.get(5)?,
            })
        })
        .expect("query normalized SQLite column inventory")
        .collect::<rusqlite::Result<Vec<_>>>()
        .expect("read normalized SQLite column inventory")
}

fn logical_value(column_name: &str, value: ValueRef<'_>) -> LogicalValue {
    match value {
        ValueRef::Null => LogicalValue::Null,
        ValueRef::Integer(value) => LogicalValue::Integer(value),
        ValueRef::Real(value) => LogicalValue::RealBits(value.to_bits()),
        ValueRef::Text(value) if column_name.ends_with("_json") => {
            let json = serde_json::from_slice(value).expect("valid JSON in SQLite JSON column");
            LogicalValue::Json(canonical_json(json))
        }
        ValueRef::Text(value) => LogicalValue::Text(value.to_vec()),
        ValueRef::Blob(value) => LogicalValue::Blob(value.to_vec()),
    }
}

fn canonical_json(value: serde_json::Value) -> String {
    serde_json::to_string(&sort_json(value)).expect("serialize canonical SQLite JSON")
}

fn sort_json(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Array(values) => {
            serde_json::Value::Array(values.into_iter().map(sort_json).collect())
        }
        serde_json::Value::Object(values) => {
            let sorted = values
                .into_iter()
                .map(|(key, value)| (key, sort_json(value)))
                .collect::<BTreeMap<_, _>>();
            serde_json::Value::Object(sorted.into_iter().collect())
        }
        value => value,
    }
}

fn quote_identifier(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

fn assert_profile_change(
    before: &LogicalTableSnapshot,
    after: &LogicalTableSnapshot,
    controlled_profile: &Profile,
) {
    let mut expected = single_row(before, "profile before pass");
    expected.insert(
        "data_json".into(),
        serialized_json_value(controlled_profile),
    );
    assert_eq!(single_row(after, "profile after pass"), expected);
}

fn assert_track_change(
    before: &LogicalTableSnapshot,
    after: &LogicalTableSnapshot,
    track_before: &TrackRecord,
    finalized_track: &TrackRecord,
) {
    assert_finalized_track_delta(track_before, finalized_track);
    let mut expected = single_row(before, "track before pass");
    expected.insert(
        "status".into(),
        LogicalValue::Text(finalized_track.status.as_str().as_bytes().to_vec()),
    );
    expected.insert("data_json".into(), serialized_json_value(finalized_track));
    expected.insert(
        "updated_at".into(),
        LogicalValue::Text(finalized_track.updated_at.as_bytes().to_vec()),
    );
    assert_eq!(single_row(after, "track after pass"), expected);
}

fn assert_finalized_track_delta(before: &TrackRecord, after: &TrackRecord) {
    let mut expected = serde_json::to_value(before).expect("serialize track before finalization");
    let actual = serde_json::to_value(after).expect("serialize track after finalization");
    for pointer in ["/status", "/integrity", "/certificate", "/updatedAt"] {
        *expected
            .pointer_mut(pointer)
            .unwrap_or_else(|| panic!("expected track field {pointer}")) = actual
            .pointer(pointer)
            .unwrap_or_else(|| panic!("finalized track field {pointer}"))
            .clone();
    }
    expected
        .pointer_mut("/audioScreening/external")
        .and_then(serde_json::Value::as_object_mut)
        .expect("expected external screening record")
        .insert(
            "configuredAtSnapshot".into(),
            actual
                .pointer("/audioScreening/external/configuredAtSnapshot")
                .expect("finalized screening configuration snapshot")
                .clone(),
        );
    assert_eq!(actual, expected, "non-finalization track data changed");
    assert_eq!(after.status, TrackStatus::Finalized);
    assert_eq!(
        after.audio_screening.external.configured_at_snapshot,
        Some(false)
    );
    assert!(after.integrity.generated && after.integrity.verified);
    assert_eq!(after.integrity.file_count, before.integrity.file_count);
    assert_eq!(after.integrity.verified_count, before.integrity.file_count);
    assert!(after.integrity.generated_at.is_none());
    assert!(after.integrity.verified_at.is_some());
    assert!(after.integrity.mismatch_files.is_empty());
    assert!(after.certificate.valid);
    assert!(after.certificate.certificate_id.is_some());
    assert!(after.certificate.finalization_snapshot_id.is_some());
    assert!(after.certificate.finalized_at.is_some());
    assert_eq!(
        after.certificate.workflow_version.as_deref(),
        Some(before.workflow_version.as_str())
    );
    assert!(after.certificate.bilingual);
    assert!(after.certificate.invalidated_at.is_none());
    assert!(after.certificate.invalidation_reason.is_none());
}

fn assert_finalization_summary_change(
    before: &LogicalTableSnapshot,
    after: &LogicalTableSnapshot,
    finalized_track: &TrackRecord,
) {
    assert!(
        before.rows.is_empty(),
        "fixture already had timestamp status"
    );
    let actual = single_row(after, "timestamp finalization summary");
    let updated_at = text_value(&actual, "updated_at");
    let certificate_id = finalized_track
        .certificate
        .certificate_id
        .as_deref()
        .expect("finalized certificate ID");
    let summary = ExternalTimestampSummary {
        status: ExternalTimestampStatus::NotRecorded,
        message: "No automatic external timestamp was requested for this finalization.".into(),
        provider: "Disabled".into(),
        record_id: None,
        updated_at: Some(updated_at.clone()),
    };
    let expected = BTreeMap::from([
        (
            "track_id".into(),
            LogicalValue::Text(finalized_track.id.as_bytes().to_vec()),
        ),
        (
            "certificate_id".into(),
            LogicalValue::Text(certificate_id.as_bytes().to_vec()),
        ),
        (
            "updated_at".into(),
            LogicalValue::Text(updated_at.into_bytes()),
        ),
        ("data_json".into(), serialized_json_value(&summary)),
    ]);
    assert_eq!(actual, expected);
}

fn single_row(table: &LogicalTableSnapshot, context: &str) -> BTreeMap<String, LogicalValue> {
    assert_eq!(table.rows.len(), 1, "expected one SQLite row: {context}");
    table
        .columns
        .iter()
        .map(|column| column.name.clone())
        .zip(table.rows[0].iter().cloned())
        .collect()
}

fn serialized_json_value(value: &impl Serialize) -> LogicalValue {
    LogicalValue::Json(canonical_json(
        serde_json::to_value(value).expect("serialize expected SQLite JSON value"),
    ))
}

fn text_value(row: &BTreeMap<String, LogicalValue>, column: &str) -> String {
    let LogicalValue::Text(value) = row.get(column).expect("SQLite text column") else {
        panic!("SQLite column {column} was not text");
    };
    String::from_utf8(value.clone()).expect("UTF-8 SQLite text value")
}

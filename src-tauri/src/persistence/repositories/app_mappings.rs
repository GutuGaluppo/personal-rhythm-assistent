use crate::persistence::error::{PersistenceError, Result};
use crate::sessions::model::Category;
use rusqlite::{params, Connection};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredMapping {
    pub bundle_id: String,
    pub application_name: Option<String>,
    pub category: Category,
}

pub fn list(conn: &Connection) -> Result<Vec<StoredMapping>> {
    let mut stmt =
        conn.prepare("SELECT bundle_id, application_name, category FROM app_category_mappings")?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, Option<String>>(1)?,
            r.get::<_, String>(2)?,
        ))
    })?;
    rows.map(|row| {
        let (bundle_id, application_name, category) = row?;
        let category = Category::parse(&category)
            .ok_or_else(|| PersistenceError::Invalid(format!("unknown category {category:?}")))?;
        Ok(StoredMapping {
            bundle_id,
            application_name,
            category,
        })
    })
    .collect()
}

pub fn upsert(
    conn: &Connection,
    bundle_id: &str,
    application_name: Option<&str>,
    category: Category,
) -> Result<()> {
    conn.execute(
        "INSERT INTO app_category_mappings (bundle_id, application_name, category)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(bundle_id) DO UPDATE SET
             application_name = COALESCE(excluded.application_name, application_name),
             category = excluded.category",
        params![bundle_id, application_name, category.as_str()],
    )?;
    Ok(())
}

pub fn delete(conn: &Connection, bundle_id: &str) -> Result<bool> {
    Ok(conn.execute(
        "DELETE FROM app_category_mappings WHERE bundle_id = ?1",
        [bundle_id],
    )? > 0)
}

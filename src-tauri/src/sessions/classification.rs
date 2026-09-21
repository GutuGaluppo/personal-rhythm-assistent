//! Manual, default-based activity classification (IMPLEMENTATION.md §9).
//! No AI classification in v0.1: an app is whatever the user (or the built-in
//! default table) says it is, and `Unknown` otherwise.

use super::model::Category;
use crate::persistence::error::Result;
use crate::persistence::repositories::{activity_events, app_mappings};
use rusqlite::Connection;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::RwLock;

/// Built-in starting points. Users can change every one of them.
const DEFAULTS: &[(&str, Category)] = &[
    ("com.microsoft.VSCode", Category::Create),
    ("com.apple.Terminal", Category::Create),
    ("com.googlecode.iterm2", Category::Create),
    ("com.apple.dt.Xcode", Category::Create),
    ("com.figma.Desktop", Category::Create),
    ("com.spotify.client", Category::Recover),
    ("com.apple.Music", Category::Recover),
    ("com.apple.iBooksX", Category::Learn),
];

pub fn default_category(bundle_id: &str) -> Option<Category> {
    DEFAULTS
        .iter()
        .find(|(id, _)| *id == bundle_id)
        .map(|(_, c)| *c)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MappingSource {
    /// Chosen by the user.
    User,
    /// Built-in default.
    Default,
    /// Nobody has classified this app; it counts as `Unknown`.
    Unset,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppMapping {
    pub bundle_id: String,
    pub application_name: String,
    pub category: Category,
    pub source: MappingSource,
}

/// The user's overrides, cached in memory so the Sessionizer can classify without
/// touching the database. The database stays the source of truth.
#[derive(Default)]
pub struct Classification {
    overrides: RwLock<HashMap<String, Category>>,
}

impl Classification {
    pub fn load(conn: &Connection) -> Result<Self> {
        let overrides = app_mappings::list(conn)?
            .into_iter()
            .map(|m| (m.bundle_id, m.category))
            .collect();
        Ok(Self {
            overrides: RwLock::new(overrides),
        })
    }

    pub fn category_for(&self, bundle_id: &str) -> Category {
        self.resolve(bundle_id).0
    }

    fn resolve(&self, bundle_id: &str) -> (Category, MappingSource) {
        if let Some(c) = self.overrides.read().unwrap().get(bundle_id) {
            return (*c, MappingSource::User);
        }
        match default_category(bundle_id) {
            Some(c) => (c, MappingSource::Default),
            None => (Category::Unknown, MappingSource::Unset),
        }
    }

    /// Remaps an app. Takes effect for the session in progress and for new sessions;
    /// sessions that already ended keep the category they had.
    pub fn set(
        &self,
        conn: &Connection,
        bundle_id: &str,
        application_name: Option<&str>,
        category: Category,
    ) -> Result<()> {
        app_mappings::upsert(conn, bundle_id, application_name, category)?;
        self.overrides
            .write()
            .unwrap()
            .insert(bundle_id.to_string(), category);
        Ok(())
    }

    /// Drops the user's choice; the app falls back to its default (or `Unknown`).
    pub fn reset(&self, conn: &Connection, bundle_id: &str) -> Result<()> {
        app_mappings::delete(conn, bundle_id)?;
        self.overrides.write().unwrap().remove(bundle_id);
        Ok(())
    }

    /// Apps to show in the mapping UI: everything seen recently plus everything the
    /// user has already classified, alphabetical.
    pub fn list_mappings(&self, conn: &Connection) -> Result<Vec<AppMapping>> {
        let mut names: HashMap<String, String> = activity_events::distinct_applications(conn)?
            .into_iter()
            .collect();
        for stored in app_mappings::list(conn)? {
            let fallback = stored
                .application_name
                .unwrap_or_else(|| stored.bundle_id.clone());
            names.entry(stored.bundle_id).or_insert(fallback);
        }

        let mut mappings: Vec<AppMapping> = names
            .into_iter()
            .map(|(bundle_id, application_name)| {
                let (category, source) = self.resolve(&bundle_id);
                AppMapping {
                    bundle_id,
                    application_name,
                    category,
                    source,
                }
            })
            .collect();
        mappings.sort_by_key(|m| (m.application_name.to_lowercase(), m.bundle_id.clone()));
        Ok(mappings)
    }
}

-- Milestone 4: user overrides for the app -> category mapping.
-- Built-in defaults live in code; a row here means the user chose the category.

CREATE TABLE app_category_mappings (
    bundle_id        TEXT PRIMARY KEY NOT NULL,
    application_name TEXT,
    category         TEXT NOT NULL
                     CHECK (category IN ('Create', 'Learn', 'Explore', 'Move', 'Life',
                                         'People', 'Recover', 'Think', 'Unknown'))
);

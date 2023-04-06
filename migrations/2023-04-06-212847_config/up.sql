CREATE TABLE config (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  serialized_page_tree BLOB NOT NULL,
  root_url TEXT NOT NULL
)

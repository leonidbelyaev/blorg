CREATE TABLE pages (
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  parent_id INTEGER,
  title TEXT NOT NULL,
  slug TEXT NOT NULL,
  html_content TEXT NOT NULL,
  FOREIGN KEY(parent_id) REFERENCES pages(id) ON DELETE CASCADE
)

CREATE TABLE pages (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  parent_id INTEGER,
  title TEXT NOT NULL,
  slug TEXT NOT NULL,
  create_time TEXT NOT NULL,
  update_time TEXT,
  html_content TEXT NOT NULL,
  markdown_content TEXT NOT NULL,
  sidebar_html_content TEXT NOT NULL,
  sidebar_markdown_content TEXT NOT NULL,
  FOREIGN KEY(parent_id) REFERENCES pages(id) ON DELETE CASCADE
)

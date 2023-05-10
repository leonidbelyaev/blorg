CREATE TABLE page_revision(
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  page_id INTEGER,
  iso_time TEXT NOT NULL,
  unix_time INTEGER NOT NULL,
  html_content TEXT NOT NULL,
  markdown_content TEXT NOT NULL,
  sidebar_html_content TEXT NOT NULL,
  sidebar_markdown_content TEXT NOT NULL,
  FOREIGN KEY (page_id) REFERENCES page(id) ON DELETE CASCADE
);

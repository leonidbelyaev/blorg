CREATE TABLE page_revision(
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  page_id INTEGER,
  time TEXT,
  html_content TEXT NOT NULL,
  markdown_content TEXT,
  sidebar_html_content TEXT NOT NULL,
  sidebar_markdown_content TEXT,
  FOREIGN KEY page_id REFERENCES page(id)
);

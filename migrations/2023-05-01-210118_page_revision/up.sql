CREATE TABLE page_revision(
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  page_id INTEGER,
  time TEXT, -- This is set to the old edit time on an edit
  markdown_content TEXT, -- These are included (atm) in every revision regardless of which was edited for simplicity
  sidebar_markdown_content TEXT,
  FOREIGN KEY page_id REFERENCES page(id)
);

CREATE TABLE comment(
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  commenter_id INTEGER NOT NULL,
  page_id INTEGER NOT NULL,
  text TEXT NOT NULL,
  FOREIGN KEY (commenter_id) REFERENCES commenter(id),
  FOREIGN KEY (page_id) REFERENCES page(id)
)

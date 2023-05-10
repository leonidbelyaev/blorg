CREATE TABLE commenter(
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  alias TEXT NOT NULL,
  password_hash TEXT -- note : this is optional for commenters
)

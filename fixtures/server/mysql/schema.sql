CREATE TABLE schema_users (
  id INTEGER PRIMARY KEY AUTO_INCREMENT,
  email VARCHAR(255) NOT NULL,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE schema_posts (
  id INTEGER PRIMARY KEY AUTO_INCREMENT,
  user_id INTEGER NOT NULL,
  CONSTRAINT schema_posts_user_fk FOREIGN KEY (user_id) REFERENCES schema_users (id)
);

CREATE INDEX schema_users_email ON schema_users (email);
CREATE TABLE schema_audit (user_id INTEGER NOT NULL);
CREATE VIEW schema_user_emails AS SELECT id, email FROM schema_users;

CREATE PROCEDURE schema_user_count()
  SELECT CAST(COUNT(*) AS SIGNED) AS value FROM schema_users;

INSERT INTO schema_users (email) VALUES ('fixture@example.org');

-- MySQL keeps a trailing semicolon in trigger bodies submitted as multiple
-- statements. End the migration here without it so mysqldump can restore the
-- trigger: https://bugs.mysql.com/bug.php?id=77856
CREATE TRIGGER schema_user_insert AFTER INSERT ON schema_users
  FOR EACH ROW BEGIN
    INSERT INTO schema_audit (user_id) VALUES (NEW.id);
  END

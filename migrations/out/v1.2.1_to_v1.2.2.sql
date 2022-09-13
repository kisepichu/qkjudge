-- Apply --
ALTER TABLE `problems` ADD COLUMN `hidden` boolean NOT NULL AFTER `path`;
INSERT INTO migrations (version) VALUES ('v1.2.2');

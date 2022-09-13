-- Apply --
ALTER TABLE `problems` ADD COLUMN `visible` boolean NOT NULL AFTER `path`;
ALTER TABLE `problems` DROP COLUMN `hidden`;
INSERT INTO migrations (version) VALUES ('v1.2.3');

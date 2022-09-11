-- Apply --
ALTER TABLE `submissions` ADD COLUMN `language_id` int(11) NOT NULL AFTER `result`;
ALTER TABLE `submissions` DROP COLUMN `language`;
ALTER TABLE `submissions` DROP COLUMN `language_version`;
INSERT INTO migrations (version) VALUES ('v1.1.0');

-- Apply --
ALTER TABLE `submissions` ADD COLUMN `language_version` varchar(100) NOT NULL AFTER `language`;
INSERT INTO migrations (version) VALUES ('v1.0.1');

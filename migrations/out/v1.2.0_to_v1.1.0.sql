-- Apply --
ALTER TABLE `problems` DROP COLUMN `title`;
ALTER TABLE `problems` DROP COLUMN `difficulty`;
ALTER TABLE `problems` DROP COLUMN `time_limit`;
ALTER TABLE `problems` DROP COLUMN `memory_limit`;
INSERT INTO migrations (version) VALUES ('v1.1.0');

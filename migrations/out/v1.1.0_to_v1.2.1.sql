-- Apply --
ALTER TABLE `problems` ADD COLUMN `title` varchar(100) NOT NULL AFTER `id`;
ALTER TABLE `problems` ADD COLUMN `author` varchar(100) NOT NULL AFTER `title`;
ALTER TABLE `problems` ADD COLUMN `difficulty` int(11) NOT NULL AFTER `author`;
ALTER TABLE `problems` ADD COLUMN `time_limit` varchar(100) NOT NULL AFTER `difficulty`;
ALTER TABLE `problems` ADD COLUMN `memory_limit` int(11) NOT NULL AFTER `time_limit`;
INSERT INTO migrations (version) VALUES ('v1.2.1');

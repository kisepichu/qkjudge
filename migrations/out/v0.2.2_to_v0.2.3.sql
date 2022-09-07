-- Apply --
ALTER TABLE `submissions` CHANGE COLUMN `source` `source` text NOT NULL;
ALTER TABLE `tasks` CHANGE COLUMN `input` `input` text NOT NULL;
ALTER TABLE `tasks` CHANGE COLUMN `output` `output` text NOT NULL;
ALTER TABLE `tasks` CHANGE COLUMN `expected` `expected` text NOT NULL;
ALTER TABLE `tasks` CHANGE COLUMN `memory` `memory` varchar(10) NOT NULL;
ALTER TABLE `tasks` CHANGE COLUMN `cpu_time` `cpu_time` varchar(10) NOT NULL;

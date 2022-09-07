-- Apply --
ALTER TABLE `submissions` ADD COLUMN `result` varchar(10) NOT NULL AFTER `testcase_num`;

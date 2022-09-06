CREATE TABLE `problems` (
  `id` int(11) NOT NULL AUTO_INCREMENT,
  `path` varchar(100) NOT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB AUTO_INCREMENT=5 DEFAULT CHARSET=utf8mb4;

CREATE TABLE `users` (
  `username` varchar(30) NOT NULL,
  `hashed_pass` varchar(200) NOT NULL,
  PRIMARY KEY (`username`)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE `submissions` (
	`id` int(11) NOT NULL AUTO_INCREMENT,
	`author` varchar(30) NOT NULL,
	`problem_id` int(11) NOT NULL,
	`language` varchar(100) NOT NULL,
	`source` text,
	CONSTRAINT fk_author FOREIGN KEY (author) REFERENCES users(username),
	CONSTRAINT fk_problem_id FOREIGN KEY (problem_id) REFERENCES problems(id),
	PRIMARY KEY (`id`)
);

CREATE TABLE `tasks` (
	`id` int(11) NOT NULL AUTO_INCREMENT,
	`submission_id` int(11) NOT NULL,
	`input` text,
	`output` text,
	`expected` text,
	`result` varchar(10) NOT NULL,
	`memory` varchar(10),
	`cpu_time` varchar(10),
	CONSTRAINT fk_submission_id FOREIGN KEY (submission_id) REFERENCES submissions(id),
	PRIMARY KEY (id)
);

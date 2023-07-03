-- Create users table
CREATE TABLE `users` (
    `id` UUID NOT NULL PRIMARY KEY,
    `username` VARCHAR(128) NOT NULL,
    `password` TEXT NOT NULL,
    UNIQUE (`username`)
);

-- Add the salt column to the users table
ALTER TABLE `users` ADD COLUMN `salt` TEXT NOT NULL;

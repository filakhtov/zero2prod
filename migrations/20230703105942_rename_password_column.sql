-- Rename the password column
ALTER TABLE `users` RENAME COLUMN `password` TO `password_hash`;

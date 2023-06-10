-- Create subscriptions table
CREATE TABLE `subscriptions` (
    `id` BINARY(16) NOT NULL PRIMARY KEY,
    `email` VARCHAR(319) NOT NULL,
    `name` TEXT NOT NULL,
    `subscribed_at` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (`email`)
);

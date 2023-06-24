-- Add migration script here
CREATE TABLE `subscription_tokens` (
    `subscription_token` VARCHAR(25) NOT NULL PRIMARY KEY,
    `subscriber_id` UUID NOT NULL REFERENCES `subscriptions`(`id`)
);

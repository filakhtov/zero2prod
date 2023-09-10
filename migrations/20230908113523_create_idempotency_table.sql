CREATE TABLE `idempotency` (
    `user_id` uuid NOT NULL,
    `idempotency_key` VARCHAR(50) NOT NULL,
    `response_status_code` SMALLINT NOT NULL,
    `response_headers` BLOB NOT NULL,
    `response_body` BLOB NOT NULL,
    `created_at` TIMESTAMP NOT NULL,
    PRIMARY KEY(`user_id`, `idempotency_key`)
);

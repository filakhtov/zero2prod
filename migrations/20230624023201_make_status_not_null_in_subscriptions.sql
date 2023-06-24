-- Add migration script here
BEGIN;
    UPDATE `subscriptions`
       SET `status` = "confirmed"
     WHERE `status` IS NULL;
    ALTER TABLE `subscriptions` MODIFY `status` VARCHAR(25) NOT NULL;
COMMIT;

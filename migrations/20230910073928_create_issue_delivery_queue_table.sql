CREATE TABLE `issue_delivery_queue` (
  `newsletter_issue_id` UUID NOT NULL,
  `subscriber_email` VARCHAR(319) NOT NULL,
  PRIMARY KEY (`newsletter_issue_id`,`subscriber_email`)
);

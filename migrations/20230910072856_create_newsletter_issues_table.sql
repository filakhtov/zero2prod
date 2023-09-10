CREATE TABLE `newsletter_issues` (
  `newsletter_issue_id` uuid NOT NULL PRIMARY KEY,
  `title` text NOT NULL,
  `text_content` text NOT NULL,
  `html_content` text NOT NULL,
  `published_at` timestamp NOT NULL
);

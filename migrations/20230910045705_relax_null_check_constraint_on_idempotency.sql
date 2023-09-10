ALTER TABLE `idempotency` MODIFY `response_status_code` SMALLINT;
ALTER TABLE `idempotency` MODIFY `response_headers` BLOB;
ALTER TABLE `idempotency` MODIFY `response_body` BLOB;

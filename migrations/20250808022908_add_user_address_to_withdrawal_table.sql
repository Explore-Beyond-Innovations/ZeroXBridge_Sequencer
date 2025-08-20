-- Add migration script here
ALTER TABLE withdrawal
ADD COLUMN user_address TEXT NOT NULL;
ß

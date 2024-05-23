-- +goose Up
-- +goose StatementBegin
ALTER TABLE assets ALTER COLUMN first_historical_data SET DATA TYPE TIMESTAMP WITH TIME ZONE;
ALTER TABLE assets ALTER COLUMN last_historical_data SET DATA TYPE TIMESTAMP WITH TIME ZONE;
-- +goose StatementEnd

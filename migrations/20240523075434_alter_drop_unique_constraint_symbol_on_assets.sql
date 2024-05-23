-- +goose StatementBegin
ALTER TABLE assets DROP CONSTRAINT assets_symbol_key;
-- +goose StatementEnd

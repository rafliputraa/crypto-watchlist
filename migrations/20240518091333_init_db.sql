-- +goose StatementBegin
CREATE TABLE users (
                       id SERIAL PRIMARY KEY,
                       username VARCHAR(255) UNIQUE NOT NULL,
                       email VARCHAR(255) UNIQUE NOT NULL,
                       created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE assets (
                        id SERIAL PRIMARY KEY,
                        name VARCHAR(255),
                        symbol VARCHAR(255) UNIQUE NOT NULL,
                        slug VARCHAR(255),
                        first_historical_data TIMESTAMP WITH TIME ZONE,
                        last_historical_data TIMESTAMP WITH TIME ZONE
);

CREATE TABLE watchlist_groups (
                                  id SERIAL PRIMARY KEY,
                                  user_id INT,
                                  name VARCHAR(255) NOT NULL,
                                  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                                  FOREIGN KEY (user_id) REFERENCES users(id)
);

CREATE TABLE watchlist (
                           group_id INT,
                           asset_id INT,
                           added_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                           PRIMARY KEY (group_id, asset_id),
                           FOREIGN KEY (group_id) REFERENCES watchlist_groups(id),
                           FOREIGN KEY (asset_id) REFERENCES assets(id)
);

CREATE INDEX idx_users_username ON users(username);
CREATE INDEX idx_assets_symbol ON assets(symbol);
CREATE INDEX idx_assets_name ON assets(name);
CREATE INDEX idx_watchlist_groups_user_id ON watchlist_groups(user_id);
CREATE INDEX idx_watchlist_group_id ON watchlist(group_id);
CREATE INDEX idx_watchlist_asset_id ON watchlist(asset_id);
CREATE INDEX idx_watchlist_group_asset ON watchlist(group_id, asset_id);
-- +goose StatementEnd

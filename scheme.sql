CREATE TABLE IF NOT EXISTS friends_ids
(
    id           BIGINT NOT NULL,
    confirmed_at BIGINT DEFAULT 0,
    PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS followers_ids
(
    id           BIGINT NOT NULL,
    confirmed_at BIGINT DEFAULT 0,
    PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS user_data
(
    id   BIGINT NOT NULL,
    data JSONB,
    PRIMARY KEY (id)
);

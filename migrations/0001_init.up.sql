CREATE TYPE moderation_action_enum AS ENUM ('APPROVED', 'REJECTED', 'NEEDS_REVIEW');

CREATE TABLE bad_words (
    id SERIAL PRIMARY KEY,
    word TEXT UNIQUE NOT NULL,
    moderation_action moderation_action_enum NOT NULL DEFAULT 'REJECTED'
);

CREATE TABLE regex_rules (
    id SERIAL PRIMARY KEY,
    pattern TEXT NOT NULL,
    moderation_action moderation_action_enum NOT NULL DEFAULT 'REJECTED',
    description TEXT
);

CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value TEXT
);

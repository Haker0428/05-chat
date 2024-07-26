-- this file is userd for postgresql database initialization
-- create user table

CREATE TABLE IF NOT EXISTS users {
    id SERIAL PRIMARY KEY,
    fullname VARCHAR(64) NOT NULL,
    email VARCHAR(64) NOT NULL,
    -- hashed argon2 password
    password VARCHAR(64) NOT NULL,
    create_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
};

-- create char type: single, group, private_channel, public_channel
CREATE TYPE chat_type AS ENUM('single', 'group', 'private_channel', 'public_channel');

-- create chat table
CREATE TABLE IF NOT EXISTS chats {
    id SERIAL PRIMARY KEY,
    name VARCHAR(128) NOT NULL UNIQUE,
    type chat_type NOT NULL,
    -- user id list
    members BIGINT[] NOT NULL
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
};

-- create message table
CREATE TABLE IF NOT EXISTS messages {
    id BIGSERIAL PRIMARY KEY,
    chat_id BIGINT NOT NULL,
    sender_id BIGINT NOT NULL,
    content TEXT NOT NULL,
    images TEXT[],
    create_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (chat_id) REFERENCES chats(id),
    FOREIGN KEY (sender_id) REFERENCES users(id),
};

-- create index for messages for chat_id and created_at order by created_at descpition
CREATE INDEX IF NOT EXISTS chat_id_created_at_index ON messages(chat_id, create_at DESC);


-- create index for messages for sender_id
CREATE INDEX IF NOT EXISTS sender_id_index ON message(sender_id);

-- create index for users for email address
CREATE INDEX IF NOT EXISTS email_index ON users(email);

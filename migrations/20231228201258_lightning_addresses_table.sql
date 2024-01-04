CREATE OR REPLACE FUNCTION update_timestamp() RETURNS TRIGGER AS $$ BEGIN NEW.updated_at = NOW();
RETURN NEW;
END;
$$ language 'plpgsql';
create table "lightning_addresses" (
    id uuid primary key default gen_random_uuid(),
    user_id varchar(255) unique not null,
    username text unique not null,
    active boolean not null default true,
    created_at timestamptz not null default current_timestamp,
    updated_at timestamptz,
    deleted_at timestamptz
);
CREATE TRIGGER update_timestamp BEFORE
UPDATE ON lightning_addresses FOR EACH ROW EXECUTE PROCEDURE update_timestamp();
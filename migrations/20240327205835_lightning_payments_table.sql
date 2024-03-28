CREATE OR REPLACE FUNCTION update_payment_timestamp() RETURNS TRIGGER AS $$ BEGIN NEW.updated_at = NOW();
RETURN NEW;
END;
$$ language 'plpgsql';
CREATE TABLE "lightning_payments" (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    lightning_address varchar(255),
    payment_hash varchar unique NOT NULL,
    error varchar,
    amount_msat bigint NOT NULL,
    fee_msat bigint,
    payment_time bigint,
    status varchar NOT NULL,
    description varchar,
    metadata varchar,
    created_at timestamptz NOT NULL DEFAULT current_timestamp,
    updated_at timestamptz
);
CREATE TRIGGER update_lightning_payment_timestamp BEFORE
UPDATE ON lightning_payments FOR EACH ROW EXECUTE PROCEDURE update_payment_timestamp();
ALTER TABLE lightning_payments
ADD CONSTRAINT fk_lightning_address FOREIGN KEY (lightning_address) REFERENCES lightning_addresses(username) ON DELETE CASCADE ON UPDATE CASCADE;
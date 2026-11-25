//! One module per API domain. Add new domains here as suites are written.

mod accounts;
mod agent_keys;
mod api_keys;
mod asset_scoping;
mod auth;
mod bitcoin;
mod cli;
mod cln_payments;
mod events;
mod guards;
mod invoices;
mod lightning;
mod ln_addresses;
mod lnurl_send;
mod mcp;
mod me;
mod oauth2;
mod payment_approvals;
mod payment_submissions;
mod payments;
mod system;
mod wallets;
mod well_known;

mod bip353;
mod bolt12;
mod notifications;
mod spending_policy;
mod withdrawal_submissions;

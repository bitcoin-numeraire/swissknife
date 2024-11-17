export type IBreezNodeInfo = {
  id: string;
  block_height: number;
  channels_balance_msat: number;
  onchain_balance_msat: number;
  pending_onchain_balance_msat: number;
  max_payable_msat: number;
  max_receivable_msat: number;
  max_single_payment_amount_msat: number;
  max_chan_reserve_msats: number;
  connected_peers: string[];
  inbound_liquidity_msats: number;
};

export type IBreezLSP = {
  id: string;
  name: string;
  pubkey: string;
  host: string;
  base_fee_msat: number;
  fee_rate: number;
  time_lock_delta: number;
  min_htlc_msat: number;
  opening_fee_params_list: OpeningFeeParams[];
};

export type OpeningFeeParams = {
  min_msat: number;
  proportional: number;
  valid_until: string;
  max_idle_time: number;
};

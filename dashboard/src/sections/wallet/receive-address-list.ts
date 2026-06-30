type ReceiveAddressListInput = {
  open: boolean;
  isAdmin: boolean;
  selectedNeedsAddress: boolean;
  addressWalletId?: string;
};

export function getReceiveAddressListState({
  open,
  isAdmin,
  addressWalletId,
  selectedNeedsAddress,
}: ReceiveAddressListInput) {
  const enabled = open && selectedNeedsAddress;
  const adminQuery =
    enabled && isAdmin && addressWalletId ? { wallet_id: addressWalletId } : undefined;

  return {
    adminQuery,
    adminEnabled: Boolean(adminQuery),
    walletEnabled: enabled && !isAdmin,
  };
}

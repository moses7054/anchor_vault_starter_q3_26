# Receipt-token vault

This program pools one classic SPL receipt mint per vault and issues transferable
SPL shares. It does not deposit into an external market or redeem receipts for
SOL; the receipt mint must already represent the market position users want to
hold in kind.

## Instructions

- `initialize`: permissionlessly creates the vault and share mint for a receipt
  mint. It rejects receipt mints that retain a freeze authority.
- `deposit(amount, min_shares_out)`: transfers receipts from the signer and
  mints pro-rata shares. The first deposit establishes the share ratio from the
  vault's post-deposit receipt balance.
- `redeem(shares, min_receipts_out)`: burns the signer's shares and transfers
  their pro-rata receipt tokens in the same transaction. The final shares
  redeem all remaining receipts, including rounding dust.

For later deposits, shares are rounded down:
`received_receipts * share_supply / vault_receipts_before`.
Redemptions are also rounded down:
`shares_burned * vault_receipts / share_supply`.
Callers should set minimum output arguments to protect against transaction
ordering and share-price changes.

## Scope and compatibility

The vault accepts classic SPL Token accounts only; Token-2022 mints and
extensions are not supported. Users exit the vault into receipt tokens, not
liquid SOL. The program-controlled PDA holds the pooled receipts, so this is a
share-based escrow vault rather than strictly non-custodial custody.

The shared-vault state and deposit/initialize interfaces are new and do not
migrate per-user native-SOL accounts. The `withdraw` and `close` instructions
remain available for exiting those legacy SOL vaults. New clients should use
`initialize`, `deposit`, and `redeem` for receipt-token vaults.

export const PROGRAM_ID = "HCM2iWcvchzhsnZnwbLdxBHWvgn4hkEyZ9CVGdgRkjPg";

/** Filled by `scripts/setup-devnet.mjs` after initialize */
export const DEVNET_CONFIG = {
  cluster: "devnet",
  rpcUrl: "https://api.devnet.solana.com",
  programId: PROGRAM_ID,
  underlyingMint: "GYLPoYJvP24nGTHJ9c2Naq2nRiyX5kL26yZ8Rh72VXUN",
  vault: "5NBgkwgAfAXMSqNKaHmFSKfA5mYeDXXYFF3LnCwp5z4b",
  shareMint: "7iM3h8AuQRQzuKRaihfaQLiYVPEsHqWnUYPuj9rYpsxm",
  vaultToken: "6fpcEsoVhEUBb7HohQYhSrVPSWw4QTWpZzTLATpqR7Vi",
  feeRecipient: "9PaARjZ7SjiBm9muBbPx5ojCRp3aBtPrtCXumKAErY3A",
  authority: "9PaARjZ7SjiBm9muBbPx5ojCRp3aBtPrtCXumKAErY3A",
  txs: {
    initialize: "2dQ7ntbsQMH3qXjVNqhYGbohesB9BHCnrZjycfH633qATwLF8Su7MfwXEeLtiZX7f8LKkkVqEz2m8xXfdp7STHHE",
    deposit: "3aE7nDLXkbvQoZx4LEWtjW25UvhvqssAxin8PaseBJNgg4x9ZwMkrMTTkc8gndk5n845bG51m6qqHgKJyvdzJuyA",
    withdraw: "42RUBtajDNat2qAWh468ytQLJiZmNqrd4HfHLuNvdBahrujZkjo29QEwtnfCCTsDLv7U1H2ETp9uvj9MvddxY46z",
  },
};

export const VAULT_SEED = Buffer.from("vault");
export const VAULT_TOKEN_SEED = Buffer.from("vault_token");
export const SHARE_MINT_SEED = Buffer.from("share_mint");

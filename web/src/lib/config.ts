export const PROGRAM_ID = "9gf1uFbnaP1LZymvDWmCW92aW7upE8KGvWudwdprq7hu";

/** Filled by `scripts/setup-devnet.mjs` after initialize */
export const DEVNET_CONFIG = {
  cluster: "devnet",
  rpcUrl: "https://api.devnet.solana.com",
  programId: PROGRAM_ID,
  underlyingMint: "AX9xgr6HoZG75bdNBjzuRatEUeVR3qXpg82ztzaT5sVL",
  vault: "ExtwCDFeuESr6DFXAMp3kHodBtRqMunJMi7iwo7LpNP2",
  shareMint: "Dbur8TuhJPqePhWvbtnTA98Cq77rY8Q1XQDp1XjzjQzY",
  vaultToken: "3PMQ22t8VMoYMRU9ZDDFNCjLdhtv4TvSpcswTuXj16xU",
  feeToken: "2zEKiHNTf9cs4V6a6FwXfFbcKtnQAc1Hrz39FCai7Rby",
  authority: "4wVjCJHwfzQpk1aQhyyVMiw6bbALC7JucwCAsEpm8WfW",
  txs: {
    initialize: "4NMqvX6fkjorw2Waty8muuMXfPm62nyJw51j3XJcbPQUwQZEAjm2XsKbYQEZrkx9Cd2FJL7uQRT6yS5XuXHiRnqB",
    deposit: "WSiPDZ4XHjAKHsJueX1AErxUiLYeajGR3rx8NWFvbHoVCEYBLWvXpuRgLneASbkTmGjr9Nyy9GnEJWTRE5FpVYT",
    withdraw: "59arLkie4JHYv8521zVUtS3PY9X4zWnWwQeXNcsWrmfUR32Y7LZwQK4uKAbKJmm81UjCC7MXjW7F7fhLJSA7mM3e",
  },
};

export const VAULT_SEED = Buffer.from("vault");
export const VAULT_TOKEN_SEED = Buffer.from("vault_token");
export const FEE_TOKEN_SEED = Buffer.from("fee_token");
export const SHARE_MINT_SEED = Buffer.from("share_mint");

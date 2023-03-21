code_id=1916
init_msg=$( jq -n \
  '{
    admin: "stars1zjjqxfqm33tz27phd0z4jyg53fv0yq7m3945le",
    bank_limit: null,
    denoms: ["ustars"],
    fees: {
      flip_bps: 300,
      holders_bps: 7000,
      reserve_bps: 1500,
      team_bps: 1500,
    },
    flips_per_block_limit: 10,
    sg721_addr: "stars19xdqd6u4n4rhtueqezh9jvc8gt8r6chmmmv9dt8d59lr03vqj8yqpma9nk",
    wallets: {
      reserve: "stars169d5rq5apfky6gxek4crxt2wxft3fjtuxkzx0n",
      team: "stars1wudpuljegu6qg833ak74g5p9fj44m5eluu3n6q",
    },
  }')

starsd tx wasm instantiate $code_id "$init_msg" --label "coin_flip" \
--admin stars1zjjqxfqm33tz27phd0z4jyg53fv0yq7m3945le --gas-prices 0.025ustars --gas auto --gas-adjustment 1.9 --from main -b block -y

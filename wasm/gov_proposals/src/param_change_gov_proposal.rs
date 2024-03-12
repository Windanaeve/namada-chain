use std::str::FromStr;

use namada_core::types::dec::Dec;
use namada_proof_of_stake::storage::{read_pos_params, write_pos_params};
use namada_tx_prelude::*;

const ATOM_DENOM: u8 = 6;
const NOBLE_DENOM: u8 = 6;
const OSMO_DENOM: u8 = 6;

#[transaction(gas = 10000)]
fn apply_tx(ctx: &mut Ctx, _tx_data: Tx) -> TxResult {
    // PoS
    // let mut pos_params = read_pos_params(ctx)?.owned;
    // pos_params.max_inflation_rate = Dec::from_str("0.1").unwrap();
    // pos_params.target_staked_ratio = Dec::from_str("0.6667").unwrap();
    // pos_params.rewards_gain_p = Dec::from_str("0.25").unwrap();
    // pos_params.rewards_gain_d = Dec::from_str("0.25").unwrap();
    // Write to storage
    // write_pos_params(ctx, &pos_params)?;

    // PGF
    // let pgf_inflation_key =
    //     governance::pgf::storage::keys::get_pgf_inflation_rate_key();
    // let pgf_inflation_rate = Dec::from_str("0.025").unwrap();
    // ctx.write(&pgf_inflation_key, pgf_inflation_rate)?;

    // Stewards
    // let steward_inflation_key =
    //     governance::pgf::storage::keys::get_steward_inflation_rate_key();
    // let steward_inflation_rate = Dec::from_str("0.001").unwrap();
    // ctx.write(&steward_inflation_key, steward_inflation_rate)?;

    // Shielded NAAN
    let native_token = ctx.get_native_token()?;
    let shielded_naan_max_rewards_key =
        token::storage_key::masp_max_reward_rate_key(&native_token);
    let shielded_naan_target_locked_amount_key =
        token::storage_key::masp_locked_amount_target_key(&native_token);
    let shielded_naan_kp_gain_key =
        token::storage_key::masp_kp_gain_key(&native_token);
    let shielded_naan_kd_gain_key =
        token::storage_key::masp_kd_gain_key(&native_token);

    // Write to storage
    ctx.write(
        &shielded_naan_max_rewards_key,
        Dec::from_str("0.01").unwrap(),
    )?;
    ctx.write(
        &shielded_naan_target_locked_amount_key,
        token::Amount::native_whole(10_000_000),
    )?;
    ctx.write(&shielded_naan_kp_gain_key, Dec::from_str("1200").unwrap())?;
    ctx.write(&shielded_naan_kd_gain_key, Dec::from_str("1200").unwrap())?;

    // Shielded ATOM
    let atom_token: Address =
        Address::decode("tnam1p5dp5qlqmm8vdkhmljpfn3h58mrs5jan2u6nl39w").unwrap();
    let shielded_atom_max_rewards_key =
        token::storage_key::masp_max_reward_rate_key(&atom_token);
    let shielded_atom_target_locked_amount_key =
        token::storage_key::masp_locked_amount_target_key(&atom_token);
    let shielded_atom_kp_gain_key =
        token::storage_key::masp_kp_gain_key(&atom_token);
    let shielded_atom_kd_gain_key =
        token::storage_key::masp_kd_gain_key(&atom_token);

    // Write to storage
    ctx.write(
        &shielded_atom_max_rewards_key,
        Dec::from_str("0.01").unwrap(),
    )?;
    ctx.write(
        &shielded_atom_target_locked_amount_key,
        token::Amount::from_uint(100_000, ATOM_DENOM).unwrap(),
    )?;
    ctx.write(&shielded_atom_kp_gain_key, Dec::from_str("120000").unwrap())?;
    ctx.write(&shielded_atom_kd_gain_key, Dec::from_str("120000").unwrap())?;

    // Shielded USDC (NOBLE)
    let noble_token: Address =
        Address::decode("tnam1p50kvf26p3zjk72vjsullh59clxu50v8rg36gu3j").unwrap();
    let shielded_noble_max_rewards_key =
        token::storage_key::masp_max_reward_rate_key(&noble_token);
    let shielded_noble_target_locked_amount_key =
        token::storage_key::masp_locked_amount_target_key(&noble_token);
    let shielded_noble_kp_gain_key =
        token::storage_key::masp_kp_gain_key(&noble_token);
    let shielded_noble_kd_gain_key =
        token::storage_key::masp_kd_gain_key(&noble_token);

    // Write to storage
    ctx.write(
        &shielded_noble_max_rewards_key,
        Dec::from_str("0.02").unwrap(),
    )?;
    ctx.write(
        &shielded_noble_target_locked_amount_key,
        token::Amount::from_uint(1_000_000, NOBLE_DENOM).unwrap(),
    )?;
    ctx.write(&shielded_noble_kp_gain_key, Dec::from_str("12000").unwrap())?;
    ctx.write(&shielded_noble_kd_gain_key, Dec::from_str("12000").unwrap())?;

    // Shielded OSMO
    let osmo_token: Address =
        Address::decode("tnam1p5mycp7z2xpfevfvdxkml95wxdf3jsg8ggms7ssd").unwrap();
    let shielded_osmo_max_rewards_key =
        token::storage_key::masp_max_reward_rate_key(&osmo_token);
    let shielded_osmo_target_locked_amount_key =
        token::storage_key::masp_locked_amount_target_key(&osmo_token);
    let shielded_osmo_kp_gain_key =
        token::storage_key::masp_kp_gain_key(&osmo_token);
    let shielded_osmo_kd_gain_key =
        token::storage_key::masp_kd_gain_key(&osmo_token);

    // Write to storage
    ctx.write(
        &shielded_osmo_max_rewards_key,
        Dec::from_str("0.01").unwrap(),
    )?;
    ctx.write(
        &shielded_osmo_target_locked_amount_key,
        token::Amount::from_uint(500_000, OSMO_DENOM).unwrap(),
    )?;
    ctx.write(&shielded_osmo_kp_gain_key, Dec::from_str("25000").unwrap())?;
    ctx.write(&shielded_osmo_kd_gain_key, Dec::from_str("25000").unwrap())?;

    Ok(())
}

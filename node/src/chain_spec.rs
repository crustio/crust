// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

use hex_literal::hex;
use sp_core::{Pair, Public, sr25519, crypto::UncheckedInto};
use crust_runtime::{
    AuthorityDiscoveryId, BalancesConfig, GenesisConfig, ImOnlineId,
    AuthorityDiscoveryConfig, SessionConfig, SessionKeys, StakerStatus,
    StakingConfig, IndicesConfig, SystemConfig, SworkConfig, SudoConfig,
    WASM_BINARY, LocksConfig
};
use cstrml_staking::Forcing;
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_consensus_babe::AuthorityId as BabeId;
use primitives::{constants::currency::CRUS, *};
use sc_service::ChainType;
use sp_runtime::{traits::{Verify, IdentifyAccount}, Perbill};
use cstrml_locks::{LockType, CRU18, CRU24, CRU24D6};

const DEFAULT_PROTOCOL_ID: &str = "cru";
// Note this is the URL for the telemetry server
//const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Helper function to generate crust session key
fn session_keys(
    grandpa: GrandpaId,
    babe: BabeId,
    im_online: ImOnlineId,
    authority_discovery: AuthorityDiscoveryId,
) -> SessionKeys {
    SessionKeys {
        grandpa,
        babe,
        im_online,
        authority_discovery,
    }
}

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

/// Helper function to generate an account ID from seed
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId where
    AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
    AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Helper function to generate stash, controller and session key from seed
pub fn get_authority_keys_from_seed(seed: &str) -> (
    AccountId,
    AccountId,
    GrandpaId,
    BabeId,
    ImOnlineId,
    AuthorityDiscoveryId,
) {
    (
        get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash", seed)),
        get_account_id_from_seed::<sr25519::Public>(seed),
        get_from_seed::<GrandpaId>(seed),
        get_from_seed::<BabeId>(seed),
        get_from_seed::<ImOnlineId>(seed),
        get_from_seed::<AuthorityDiscoveryId>(seed),
    )
}

/// The `ChainSpec parametrised for crust runtime`.
pub type CrustChainSpec = sc_service::GenericChainSpec<GenesisConfig>;
type AccountPublic = <Signature as Verify>::Signer;

/// Crust development config (single validator Alice)
pub fn development_config() -> Result<CrustChainSpec, String> {
    let wasm_binary = WASM_BINARY.ok_or("Local test wasm not available")?;

    Ok(CrustChainSpec::from_genesis(
        "Development",
        "dev",
        ChainType::Development,
        move || testnet_genesis(
            wasm_binary,
            vec![
                get_authority_keys_from_seed("Alice")
            ],
            get_account_id_from_seed::<sr25519::Public>("Alice"),
            vec![
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                get_account_id_from_seed::<sr25519::Public>("Bob"),
                get_account_id_from_seed::<sr25519::Public>("Charlie"),
                get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
                get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
                get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
            ],
            true,
        ),
        vec![],
        None,
        Some(DEFAULT_PROTOCOL_ID),
        None,
        Default::default()
    ))
}

/// Crust local testnet config (multi-validator Alice + Bob)
pub fn local_testnet_config() -> Result<CrustChainSpec, String> {
    let wasm_binary = WASM_BINARY.ok_or("Local test wasm not available")?;

    Ok(CrustChainSpec::from_genesis(
        "Local Testnet",
        "local_testnet",
        ChainType::Local,
        move || testnet_genesis(
            wasm_binary,
            vec![
                get_authority_keys_from_seed("Alice"),
                get_authority_keys_from_seed("Bob"),
            ],
            get_account_id_from_seed::<sr25519::Public>("Alice"),
            vec![
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                get_account_id_from_seed::<sr25519::Public>("Bob"),
                get_account_id_from_seed::<sr25519::Public>("Charlie"),
                get_account_id_from_seed::<sr25519::Public>("Dave"),
                get_account_id_from_seed::<sr25519::Public>("Eve"),
                get_account_id_from_seed::<sr25519::Public>("Ferdie"),
                get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
                get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
                get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
                get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
                get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
                get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
            ],
            true,
        ),
        vec![],
        None,
        Some(DEFAULT_PROTOCOL_ID),
        None,
        Default::default()
    ))
}

/// Crust rocky(aka. internal testnet) config
pub fn rocky_config() -> Result<CrustChainSpec, String> {
    CrustChainSpec::from_json_bytes(&include_bytes!("../res/rocky.json")[..])
}

/// Crust maxwell(aka. open testnet) config
pub fn maxwell_config() -> Result<CrustChainSpec, String> {
    CrustChainSpec::from_json_bytes(&include_bytes!("../res/maxwell.json")[..])
}

/// Crust rocky staging config
pub fn rocky_staging_config() -> Result<CrustChainSpec, String> {
    let wasm_binary = WASM_BINARY.ok_or("Rocky wasm not available")?;

    Ok(CrustChainSpec::from_genesis(
        "Crust Rocky",
        "crust_rocky",
        ChainType::Live,
        move || rocky_staging_testnet_config_genesis(wasm_binary),
        vec![],
        None,
        Some(DEFAULT_PROTOCOL_ID),
        None,
        Default::default()
    ))
}

/// Crust maxwell staging config
pub fn maxwell_staging_config() -> Result<CrustChainSpec, String> {
    let wasm_binary = WASM_BINARY.ok_or("Maxwell wasm not available")?;

    Ok(CrustChainSpec::from_genesis(
        "Crust Maxwell",
        "crust_maxwell",
        ChainType::Live,
        move || maxwell_staging_testnet_config_genesis(wasm_binary),
        vec![],
        None,
        Some(DEFAULT_PROTOCOL_ID),
        None,
        Default::default()
    ))
}

/// Crust mainnet staging config
pub fn mainnet_staging_config() -> Result<CrustChainSpec, String> {
    let wasm_binary = WASM_BINARY.ok_or("Mainnet wasm not available")?;

    Ok(CrustChainSpec::from_genesis(
        "Crust",
        "crust",
        ChainType::Live,
        move || mainnet_staging_testnet_config_genesis(wasm_binary),
        vec![],
        None,
        Some(DEFAULT_PROTOCOL_ID),
        None,
        Default::default()
    ))
}

/// The genesis spec of crust dev/local test network
fn testnet_genesis(
    wasm_binary: &[u8],
    initial_authorities: Vec<(
        AccountId,
        AccountId,
        GrandpaId,
        BabeId,
        ImOnlineId,
        AuthorityDiscoveryId,
    )>,
    _root_key: AccountId,
    endowed_accounts: Vec<AccountId>,
    _enable_println: bool,
) -> GenesisConfig {
    const ENDOWMENT: u128 = 1_000_000 * CRUS;
    const STASH: u128 = 20_000 * CRUS;
    GenesisConfig {
        pallet_sudo: Some(SudoConfig {
            key: endowed_accounts[0].clone(),
        }),
        frame_system: Some(SystemConfig {
            code: wasm_binary.to_vec(),
            changes_trie_config: Default::default(),
        }),
        balances_Instance1: Some(BalancesConfig {
            balances: endowed_accounts
                .iter()
                .cloned()
                .map(|k| (k, ENDOWMENT))
                .collect(),
        }),
        pallet_indices: Some(IndicesConfig {
            indices: vec![],
        }),
        pallet_session: Some(SessionConfig {
            keys: initial_authorities
                .iter()
                .map(|x| {
                    (
                        x.0.clone(),
                        x.0.clone(),
                        session_keys(x.2.clone(), x.3.clone(), x.4.clone(), x.5.clone()),
                    )
                })
                .collect::<Vec<_>>(),
        }),
        staking: Some(StakingConfig {
            validator_count: 4,
            minimum_validator_count: 1,
            stakers: initial_authorities
                .iter()
                .map(|x| (x.0.clone(), x.1.clone(), STASH, StakerStatus::Validator))
                .collect(),
            invulnerables: initial_authorities.iter().map(|x| x.0.clone()).collect(),
            force_era: Forcing::NotForcing,
            slash_reward_fraction: Perbill::from_percent(10),
            ..Default::default()
        }),
        market: Some(Default::default()),
        pallet_babe: Some(Default::default()),
        pallet_grandpa: Some(Default::default()),
        pallet_im_online: Some(Default::default()),
        pallet_authority_discovery: Some(AuthorityDiscoveryConfig {
            keys: vec![]
        }),
        swork: Some(SworkConfig {
            init_codes: vec![]
        }),
        locks: Some(LocksConfig {
            genesis_locks: vec![]
        }),
        pallet_treasury: Some(Default::default()),
    }
}

/// The genesis spec of crust rocky test network
fn rocky_staging_testnet_config_genesis(wasm_binary: &[u8]) -> GenesisConfig {
    // subkey inspect "$SECRET"
    let endowed_accounts: Vec<AccountId> = vec![
        // 5Ctacdhp72PDbXs4h2Qdmc5d6J9uwg1zPu5Z2aFPUBaUKGwH
        hex!["248366954d7c2003b05a731a050fe82dae98c36c468f99cfd7e1d37b4b1e4943"].into(),
        // 5EAEWGZDwj9Ext8VcG6W892x2kxNaKCxzVFsrvkfKSLyekSF
        hex!["5cafdd8022ed42511540162e489bdbadc369602661548943f013c40c5c85ad07"].into(),
        // 5H5rWwbS5bjDZ4JRvBc78dgobkzzseGWshuRhBKDEyaNeCeW
        hex!["de0d907f35d97a5fd4db57575e779e061793a53a1968004574204394c20da472"].into(),
        // 5DhuNPjF1cAEg2nXoBhjvutHqE9o9UEXxeZStL4CmBJ5hafT
        hex!["489b1160b71714461afa2134ec2c2a942c9db2de76b6d4ad6ad19c02feafdd22"].into(),
        // 5FqR1DujpqXasMtMw71Jti5dfKaPVrKp7MrVRtHqESrbMcCP
        hex!["a6ce03517e31da7c5c3421d73bd13305fa07b109226eaa684324154ae1407f23"].into(),
        // 5HY4tsWDtD9jHersn3vBRyW9gFQSTSFy9Z6SehaxuxJ6qC7r
        hex!["f20b9d0389123001035961e7d0a8430745fcf6af9be082d713d07048e9bbf439"].into()
    ];

    // for i in 1; do for j in {stash, controller}; do subkey inspect "$SECRET//$i//$j"; done; done
    // for i in 1; do for j in grandpa; do subkey --ed25519 inspect "$SECRET//$i//$j"; done; done
    // for i in 1; do for j in babe; do subkey --sr25519 inspect "$SECRET//$i//$j"; done; done
    // for i in 1; do for j in im_online; do subkey --sr25519 inspect "$SECRET//$i//$j"; done; done
    // for i in 1; do for j in authority_discovery; do subkey --sr25519 inspect "$SECRET//$i//$j"; done; done
    let initial_authorities: Vec<(
        AccountId,
        AccountId,
        GrandpaId,
        BabeId,
        ImOnlineId,
        AuthorityDiscoveryId,
    )> = vec![(
        // 5EAEWGZDwj9Ext8VcG6W892x2kxNaKCxzVFsrvkfKSLyekSF
        hex!["5cafdd8022ed42511540162e489bdbadc369602661548943f013c40c5c85ad07"].into(),
        // 5Ctacdhp72PDbXs4h2Qdmc5d6J9uwg1zPu5Z2aFPUBaUKGwH
        hex!["248366954d7c2003b05a731a050fe82dae98c36c468f99cfd7e1d37b4b1e4943"].into(),
        // 5EaoUzR1rFsH1eDpyYZzGQDTAZnb4ZggeJT61Kkp9PfwuS2e
        hex!["6f6cc4c65692e4f89f17e03ed77bb5f3ea3422e7e13ecd98e1a79ee3de36637e"].unchecked_into(),
        // 5EAEWGZDwj9Ext8VcG6W892x2kxNaKCxzVFsrvkfKSLyekSF
        hex!["5cafdd8022ed42511540162e489bdbadc369602661548943f013c40c5c85ad07"].unchecked_into(),
        // 5EAEWGZDwj9Ext8VcG6W892x2kxNaKCxzVFsrvkfKSLyekSF
        hex!["5cafdd8022ed42511540162e489bdbadc369602661548943f013c40c5c85ad07"].unchecked_into(),
        // 5EAEWGZDwj9Ext8VcG6W892x2kxNaKCxzVFsrvkfKSLyekSF
        hex!["5cafdd8022ed42511540162e489bdbadc369602661548943f013c40c5c85ad07"].unchecked_into(),
    )];

    // Constants
    const ENDOWMENT: u128 = 2_500_000 * CRUS;
    const STASH: u128 = 1_250_000 * CRUS;

    GenesisConfig {
        pallet_sudo: Some(SudoConfig {
            key: endowed_accounts[0].clone(),
        }),
        frame_system: Some(SystemConfig {
            code: wasm_binary.to_vec(),
            changes_trie_config: Default::default(),
        }),
        balances_Instance1: Some(BalancesConfig {
            balances: endowed_accounts
                .iter()
                .cloned()
                .map(|k| (k, ENDOWMENT))
                .collect(),
        }),
        pallet_indices: Some(IndicesConfig {
            indices: vec![],
        }),
        pallet_session: Some(SessionConfig {
            keys: initial_authorities.iter().map(|x| (
                x.0.clone(),
                x.0.clone(),
                session_keys(x.2.clone(), x.3.clone(), x.4.clone(), x.5.clone()),
            )).collect::<Vec<_>>(),
        }),
        staking: Some(StakingConfig {
            validator_count: 10,
            minimum_validator_count: 1,
            stakers: initial_authorities
                .iter()
                .map(|x| (x.0.clone(), x.1.clone(), STASH, StakerStatus::Validator))
                .collect(),
            invulnerables: initial_authorities.iter().map(|x| x.0.clone()).collect(),
            force_era: Forcing::NotForcing,
            slash_reward_fraction: Perbill::from_percent(10),
            ..Default::default()
        }),
        market: Some(Default::default()),
        pallet_babe: Some(Default::default()),
        pallet_grandpa: Some(Default::default()),
        pallet_im_online: Some(Default::default()),
        pallet_authority_discovery: Some(AuthorityDiscoveryConfig {
            keys: vec![]
        }),
        swork: Some(SworkConfig {
            init_codes: vec![]
        }),
        locks: Some(LocksConfig {
            genesis_locks: vec![]
        }),
        pallet_treasury: Some(Default::default()),
    }
}

/// The genesis spec of crust maxwell test network
fn maxwell_staging_testnet_config_genesis(wasm_binary: &[u8]) -> GenesisConfig {
    // subkey inspect "$SECRET"
    let endowed_accounts: Vec<AccountId> = vec![
        // 5Dhss1MkoP1dwPgQABGJEabTcSb6wacD1zQBuJCa6FJdQupX
        hex!["4895fefce14bb3aee9e55cf7c5adde4bcd1fdbd5957d736a5f6e641a956c750f"].into(),
        // 5EReCPsRWBeKghGAB871TtnvsyUxbHSK1Cah6uUgdpurijoe
        hex!["68704cd3ebb09909fa39c8d0b3f5561a0e7e9e1ee15ad38e187b4e6a6618d352"].into(),
        // 5GYhrGQEz82p75LjvBYXF6HgPbwuFCATjC516emaFnGxW36V
        hex!["c64bc822a3d7c5a656e82ccd3c84d9fc61e09de146ba56b1e110ea2dacbc5418"].into(),
        // 5GBgu8tKRqQW5jVw1g99oGu9jZmNBzMW763Co6DNRikQcD8g
        hex!["b6446db7bbd222dc895e4660b4ece95722c5d5fe9b642e4fa5681fc48c653326"].into(),
        // 5DP1oCciSLUeDjAMaM1ySVk3aes4DFFWX4jAQvn7ToyoeikX
        hex!["3a330af995b7ced720718be5e93a97f7def2b1815a00bf66762d88508b0dd750"].into(),
        // 5Ea7YkWAr6fQJVXukPttx4Y5HjKEjLEQAKj41RKMU6YNpagn
        hex!["6ee655bb1d454925362dc253db84530d452bda9b36eb2fa03416cdd267dccf02"].into(),
        // 5D7scpwpUtM3EW8rUDEGdgdF1jqASSCovLt8a79G9SEiGDET
        hex!["2ea6d44e805bb15ced3d59c8f955cfc77a15324b8d33c212de62a4dd9469ff62"].into(),
        // 5E9Tsxb8Cg8hf4NryCNiph5rjRAhExVLj8iP8EwMaabaEpqU
        hex!["5c19a40010e0e65db4c96ea3131b7aeb151fe571bfc6230fe06001645c76b756"].into()
    ];

    // for i in 1; do for j in {stash, controller}; do subkey inspect "$SECRET//$i//$j"; done; done
    // for i in 1; do for j in grandpa; do subkey --ed25519 inspect "$SECRET//$i//$j"; done; done
    // for i in 1; do for j in babe; do subkey --sr25519 inspect "$SECRET//$i//$j"; done; done
    // for i in 1; do for j in im_online; do subkey --sr25519 inspect "$SECRET//$i//$j"; done; done
    // for i in 1; do for j in authority_discovery; do subkey --sr25519 inspect "$SECRET//$i//$j"; done; done
    let initial_authorities: Vec<(
        AccountId,
        AccountId,
        GrandpaId,
        BabeId,
        ImOnlineId,
        AuthorityDiscoveryId,
    )> = vec![(
        // 5EReCPsRWBeKghGAB871TtnvsyUxbHSK1Cah6uUgdpurijoe
        hex!["68704cd3ebb09909fa39c8d0b3f5561a0e7e9e1ee15ad38e187b4e6a6618d352"].into(),
        // 5Dhss1MkoP1dwPgQABGJEabTcSb6wacD1zQBuJCa6FJdQupX
        hex!["4895fefce14bb3aee9e55cf7c5adde4bcd1fdbd5957d736a5f6e641a956c750f"].into(),
        // 5CYdoFnHGT1wMwmEpEmShmNL1sgFKjuqjHnJQ7WGFUCyVocd
        hex!["154d354140ec66ba3002562af519fdbc3b8ee9a0401d4efb53a2ae821e1df2fc"].unchecked_into(),
        // 5EReCPsRWBeKghGAB871TtnvsyUxbHSK1Cah6uUgdpurijoe
        hex!["68704cd3ebb09909fa39c8d0b3f5561a0e7e9e1ee15ad38e187b4e6a6618d352"].unchecked_into(),
        // 5EReCPsRWBeKghGAB871TtnvsyUxbHSK1Cah6uUgdpurijoe
        hex!["68704cd3ebb09909fa39c8d0b3f5561a0e7e9e1ee15ad38e187b4e6a6618d352"].unchecked_into(),
        // 5EReCPsRWBeKghGAB871TtnvsyUxbHSK1Cah6uUgdpurijoe
        hex!["68704cd3ebb09909fa39c8d0b3f5561a0e7e9e1ee15ad38e187b4e6a6618d352"].unchecked_into(),
    )];

    // Constants
    const ENDOWMENT: u128 = 2_500_000 * CRUS;
    const STASH: u128 = 1_250_000 * CRUS;

    GenesisConfig {
        pallet_sudo: Some(SudoConfig {
            key: endowed_accounts[0].clone(),
        }),
        frame_system: Some(SystemConfig {
            code: wasm_binary.to_vec(),
            changes_trie_config: Default::default(),
        }),
        balances_Instance1: Some(BalancesConfig {
            balances: endowed_accounts
                .iter()
                .cloned()
                .map(|k| (k, ENDOWMENT))
                .collect(),
        }),
        pallet_indices: Some(IndicesConfig {
            indices: vec![],
        }),
        pallet_session: Some(SessionConfig {
            keys: initial_authorities.iter().map(|x| (
                x.0.clone(),
                x.0.clone(),
                session_keys(x.2.clone(), x.3.clone(), x.4.clone(), x.5.clone()),
            )).collect::<Vec<_>>(),
        }),
        staking: Some(StakingConfig {
            validator_count: 15,
            minimum_validator_count: 1,
            stakers: initial_authorities
                .iter()
                .map(|x| (x.0.clone(), x.1.clone(), STASH, StakerStatus::Validator))
                .collect(),
            invulnerables: initial_authorities.iter().map(|x| x.0.clone()).collect(),
            force_era: Forcing::NotForcing,
            slash_reward_fraction: Perbill::from_percent(10),
            ..Default::default()
        }),
        market: Some(Default::default()),
        pallet_babe: Some(Default::default()),
        pallet_grandpa: Some(Default::default()),
        pallet_im_online: Some(Default::default()),
        pallet_authority_discovery: Some(AuthorityDiscoveryConfig {
            keys: vec![]
        }),
        swork: Some(SworkConfig {
            init_codes: vec![]
        }),
        locks: Some(LocksConfig {
            genesis_locks: vec![]
        }),
        pallet_treasury: Some(Default::default()),
    }
}

/// The genesis spec of crust mainnet test network
fn mainnet_staging_testnet_config_genesis(wasm_binary: &[u8]) -> GenesisConfig {
    // subkey inspect "$SECRET"
    let endowed_accounts: Vec<AccountId> = vec![
        // cTLTsgfb9aCwEbPrkErFzP8ijhwfkFDTvqBxXa17WQ6dcai7N
        hex!["b6763bf3933231c6bf164e33339ae8a8bfcf6cc08477e47816af30a989810d79"].into(),
        // cTMKoe6bJL1Wud7w9z2mTW1nQwJFspudz68H7W8K1TSXFzzhw
        hex!["1c37d81ef1ebfc2953216a566cf490c7d53db3adaa4aeab15acc4ca2d6577a1d"].into(),
    ];

    // for i in 1; do for j in {stash, controller}; do subkey inspect "$SECRET//$i//$j"; done; done
    // for i in 1; do for j in grandpa; do subkey --ed25519 inspect "$SECRET//$i//$j"; done; done
    // for i in 1; do for j in babe; do subkey --sr25519 inspect "$SECRET//$i//$j"; done; done
    // for i in 1; do for j in im_online; do subkey --sr25519 inspect "$SECRET//$i//$j"; done; done
    // for i in 1; do for j in authority_discovery; do subkey --sr25519 inspect "$SECRET//$i//$j"; done; done
    let initial_authorities: Vec<(
        AccountId,
        AccountId,
        GrandpaId,
        BabeId,
        ImOnlineId,
        AuthorityDiscoveryId,
    )> = vec![(
        // cTMKoe6bJL1Wud7w9z2mTW1nQwJFspudz68H7W8K1TSXFzzhw
        hex!["1c37d81ef1ebfc2953216a566cf490c7d53db3adaa4aeab15acc4ca2d6577a1d"].into(),
        // cTMedEiTGjKYJw8U1P9R7wPPS3Fwe5KCbiWy5pAkza7CPKubM
        hex!["683a26127e98e79c45f1bb08c6941179e2932b416017c6ac6cb0fd5665d7354e"].into(),
        // cTMKoe6bJL1Wud7w9z2mTW1nQwJFspudz68H7W8K1TSXFzzhw --ed25519
        hex!["ad9996dcf1123ea5a1fc134a2124b958f2faeb16dacebf2923192702b33a8a0c"].unchecked_into(),
        // cTMKoe6bJL1Wud7w9z2mTW1nQwJFspudz68H7W8K1TSXFzzhw --sr25519
        hex!["1c37d81ef1ebfc2953216a566cf490c7d53db3adaa4aeab15acc4ca2d6577a1d"].unchecked_into(),
        // cTMKoe6bJL1Wud7w9z2mTW1nQwJFspudz68H7W8K1TSXFzzhw --sr25519
        hex!["1c37d81ef1ebfc2953216a566cf490c7d53db3adaa4aeab15acc4ca2d6577a1d"].unchecked_into(),
        // cTMKoe6bJL1Wud7w9z2mTW1nQwJFspudz68H7W8K1TSXFzzhw --sr25519
        hex!["1c37d81ef1ebfc2953216a566cf490c7d53db3adaa4aeab15acc4ca2d6577a1d"].unchecked_into(),
    )];

    let initial_locks: Vec<(
        AccountId,
        Balance,
        LockType
    )> = vec![
        // DCFs
        (hex!["58b687e32a19ed0fa306f21aeed57e9209b3af4cc22692ff18d6281f4a0d4228"].into(), 1_000_000 * CRUS, CRU24),
        (hex!["56077d36e45bb7c2c8e49a4f85ee7b026572f34eec66ad786ab96f4965d14c0b"].into(), 1_000_000 * CRUS, CRU24),
        (hex!["ea343ab04b5ee22196fe4aefb976667c1a057683d6d91b9f9d5d1d2ab352c47c"].into(), 1_000_000 * CRUS, CRU24),
        (hex!["18a77325b51b0753b78c455d28d8e3716ad624dfa6aed0d4116f625d97f12546"].into(), 1_000_000 * CRUS, CRU24),
        // Teams
        (hex!["14750f6380c863f2c3ae630741eec1465c449987523cc26446fa8ddc22e39c4b"].into(), 1_000_000 * CRUS, CRU24),
        (hex!["66ab69cbe53e67572540b82ba2ae82b1fce0311929ab8271ab7c06de444eeb41"].into(), 1_000_000 * CRUS, CRU24),
        (hex!["da9576706126661eef75269446d09bddd0355dc800ba36f740519b9119431d56"].into(), 1_000_000 * CRUS, CRU24),
        (hex!["160bcfaaf00fc11f9aa146e220c215afd755fa4f079953ed6e8ea9c8d289737a"].into(), 1_000_000 * CRUS, CRU24),
        // Seeds
        (hex!["b61f792c7252e1efbf29bbfa0f71156510659463e72368a74bc22bec2cf6480d"].into(), 200_000 * CRUS, CRU24D6),
        (hex!["1ccaa68ba65b0f53c476e5b06adb2f95580de859c278c810272c99cd87da9556"].into(), 200_000 * CRUS, CRU24D6),
        (hex!["4c1190cbdc8b63c92521a21fea95b858230f6effc37d295b506b8e2f9a83ba11"].into(), 300_000 * CRUS, CRU24D6),
        (hex!["26f88714bd5359b9833b7aca8e1d3010b07ad11dab85caa568b67c56560e8870"].into(), 300_000 * CRUS, CRU24D6),

        // CRU18
        (hex!["ee1ee6dd21b26f2b28d8054b3a8900f45f2affc73956064396df285796cd084f"].into(), 17000000000000000, CRU18),
        (hex!["16264ad5ab839bfde1c6200cbe59281569b864d7c9ca8bfe901b7f16a912d717"].into(), 51000000000000000, CRU18),
        (hex!["cc8b68461b9ee769ed7869559b8b100681ef8fdefabd37acf4c1eef346295237"].into(), 666308893234265, CRU18),
        (hex!["281a1592e998b38b5aecb7bdf676ad3978e3fecaa468eeaaa0d5698bd44a4c12"].into(), 8500000000000000, CRU18),
        (hex!["e40d727169ebbfbb7b3fa41b9cf5871f8589ea13bfbffd47e152687fdca07a5c"].into(), 46536297545267, CRU18),
        (hex!["cc8b68461b9ee769ed7869559b8b100681ef8fdefabd37acf4c1eef346295237"].into(), 6979645271329300, CRU18),
        (hex!["b65bca1366233d5d1acc11b780aeb5ea52288887cae5b618e4a13ee2dbd03001"].into(), 223431783505214, CRU18),
        (hex!["cc8b68461b9ee769ed7869559b8b100681ef8fdefabd37acf4c1eef346295237"].into(), 3818327624152650, CRU18),
        (hex!["8a6bcc1acfd2f103ece563b953540012626fa31f89018da78c1772b2eb64d30a"].into(), 8330366626127, CRU18),
        (hex!["7a1e2e0707cfb55a4e799e9cecc0d8ae46331ec6512014ee06582637c6178279"].into(), 17000000000000000, CRU18),
        (hex!["761b92e402cd612928882baa7709acd4692d8cc1b7e7cb44f63a4798cb0e002a"].into(), 144415425874436, CRU18),
        (hex!["9a2b2224d8a6944df8f70196c856ae141ecf1aeb3de10d54461317f2fdeae21b"].into(), 42500000000000000, CRU18),
        (hex!["ac1582ce03e44347f849d6b6df380e9c9ba99e6686cde9bcb22756544ac3d676"].into(), 73294866590857, CRU18),
        (hex!["cc8b68461b9ee769ed7869559b8b100681ef8fdefabd37acf4c1eef346295237"].into(), 2200666628225430, CRU18),
        (hex!["02ce32e9ef115ba1846d8a2d330527eb7e83f94f2a633e7533ed2e0be6c9181f"].into(), 1275000000000000, CRU18),
        (hex!["b0e06b4a9265f29e62969a52d7fea722a11e89631ee5d734eb0df6db5e14c65c"].into(), 42500000000000000, CRU18),
        (hex!["123d0757f138e36b3fbab3e636a1e5a1d4576ac0512facd9423f9f494d269508"].into(), 623273134760000, CRU18),
        (hex!["be9936438ccfa98d0eb084b4a8599a4a7d5d0224501c12decf6bdd9df38ca034"].into(), 42500000000000000, CRU18),
        (hex!["f4ee1bb49f4bbf9230ae07af2d12f3ccd06cf792599ad114056e150d03cad20f"].into(), 11752100000000000, CRU18),
        (hex!["bcc42d1b7c264f3aa291d747947a2b76ed44b8d9da12d269a83145d108565863"].into(), 2975000000000000, CRU18),
        (hex!["d45ab91ac3891e7b266b56328b95d7dd3332afff4ef4c64d15f0bf93d79bd26e"].into(), 11029482169969, CRU18),
        (hex!["16c8dea9f2c2d16707a3e6e1c3ffb02e1c81302b6e6ca4d105cb60a1dfd8db2b"].into(), 106343093761000, CRU18),
        (hex!["c4253b5129e6ecef311793482e0b98339d0e8192903d4a1c750eedeac949145b"].into(), 47859019718324, CRU18),
        (hex!["cc8b68461b9ee769ed7869559b8b100681ef8fdefabd37acf4c1eef346295237"].into(), 1396425845700000, CRU18),
        (hex!["1a078cbe21bcc750120c0f50fc5c13ac0181aa86508c8a456e15b83e98a91a73"].into(), 255000000000000000, CRU18),
        (hex!["368539ec0d760b4a476dc7ff62f6dce64e5e73818614863ea976ee0f3f9c4417"].into(), 61619451498167, CRU18),
        (hex!["a26c5f5954d5cc38266264df26400819b46416128d7336d6913153502373906e"].into(), 925620072191128, CRU18),
        (hex!["6ee04f28d9368748ff90b7cdf45719b0743710d7905f14dd21816ca54106b53a"].into(), 382445673058541, CRU18),
        (hex!["805826cd84a05f8296aabb2af50a7d7cbd1f1e39e524deb0031270a9bfc1373d"].into(), 4250000000000000, CRU18),
        (hex!["e208e48dcbfcfe4d16532d242f4c1bc73573dff6428136f83f2fdcd68ace0526"].into(), 514557573810000, CRU18),
        (hex!["2c4dfd315a93a167ecf05aee27b7aa2ec482b8380c989d3196ff41775dd3334f"].into(), 39999300000000000, CRU18),
        (hex!["cc8b68461b9ee769ed7869559b8b100681ef8fdefabd37acf4c1eef346295237"].into(), 548402623120000, CRU18),
        (hex!["e208e48dcbfcfe4d16532d242f4c1bc73573dff6428136f83f2fdcd68ace0526"].into(), 109563560249315, CRU18),
        (hex!["74e053613e5d9160f36cbd815766ad4d0f67100fa8c819f110bb7a7f77a3b966"].into(), 85000000000000000, CRU18),
        (hex!["b0810321cf2ffa80dbbd5d390ed7f8d2d59425eea9e7acfcbd71bd215db3086a"].into(), 393146594037363, CRU18),
        (hex!["96783c6076d1bb7d99dc75c15ce647a8e2885392ec2f3105639471aab43c3005"].into(), 4250000000000000, CRU18),
        (hex!["d0d194cb1f81e8dcfc5400b8c51dfa7ba18d0f5abd79ce8a0567fb02c9b8f953"].into(), 1700000000000000, CRU18),
        (hex!["fc7c21b96aad844d0cf897d615c7e15e3d36f536c2e373eaba5c066af6837d69"].into(), 187985069697214, CRU18),
        (hex!["8e171feda38db447e74c2d5254adb443035be225b906ea86320509a0c6368059"].into(), 63590114187650, CRU18),
        (hex!["b2d7adadba84b4729868714e2e30b851544b92aae17471945fc9515a3f20b810"].into(), 10500000000000000, CRU18),
        (hex!["e208e48dcbfcfe4d16532d242f4c1bc73573dff6428136f83f2fdcd68ace0526"].into(), 1017401933320000, CRU18),
        (hex!["4a67d080c10920812dede82a47b1df1e7fd83f4051c4936349f629a700277713"].into(), 17101861057170, CRU18),
        (hex!["e6f293a5c0ecf3030dc15d94e13714ddb19cd7f8e87ecce762d95e6151018d7a"].into(), 316467687790000, CRU18),
        (hex!["8e171feda38db447e74c2d5254adb443035be225b906ea86320509a0c6368059"].into(), 80200524304035, CRU18),
        (hex!["cc8b68461b9ee769ed7869559b8b100681ef8fdefabd37acf4c1eef346295237"].into(), 871971685493839, CRU18),
        (hex!["068b174df69debccede0e149aa156fa0430a397c67c05148770b180ce5e14b33"].into(), 360178430202686, CRU18),
        (hex!["eea06f1c016348e72d316d02d0f7c12226289dcc73c8578e2bd5a8b82bc9045d"].into(), 4250000000000000, CRU18),
        (hex!["7a5cd57e96be2edec20923dfa36bae8f8beaf5b3933265ac16371604435bd726"].into(), 39999300000000000, CRU18),
        (hex!["0ec38d2b64952200293e4659d7d2106abe5ed2e6a47c1ddfda98b02e1d39a271"].into(), 40417480777000, CRU18),
        (hex!["f4e89f5a96e9d3df8e10f314c48583b27c22198ca61211fd0b24f5fb9b2b9a5d"].into(), 5197610000000000, CRU18),
        (hex!["cc8b68461b9ee769ed7869559b8b100681ef8fdefabd37acf4c1eef346295237"].into(), 5620607202900260, CRU18),
        (hex!["90c0abf7ebca2a1dbc34d445d17b594c300154fb74155268d7c9efe9919e442a"].into(), 287882014493586, CRU18),
        (hex!["4c03b54f6f112af3ce580596114624652954cd62bc36b67d01488cb4ccedbf32"].into(), 42500000000000000, CRU18),
        (hex!["182400644f4780a65a43e00f9630152fe0ab2323d0dacd04e808ceccf462f416"].into(), 55250000000000000, CRU18),
        (hex!["18fdd7b6f9ffb16214e486d871df13793d07236bbe6ba8f4411a9c0509dc983d"].into(), 1555728127000, CRU18),
        (hex!["92ae3c0648e4b8c3db352ffe400043eb5030678243b1f2ac14e910b5bbf88334"].into(), 1088141578977900, CRU18),
        (hex!["368539ec0d760b4a476dc7ff62f6dce64e5e73818614863ea976ee0f3f9c4417"].into(), 21232410350559, CRU18),
        (hex!["76ed9e5effe1d222896a31500ac0418685efc36b37993d651b3b34fb4c84c03e"].into(), 8500000000000000, CRU18),
        (hex!["c058aaa9454831e3d6f4428509c17fa6cb0439ab30a21b1458c2264aab35666b"].into(), 2458171182260000, CRU18),
        (hex!["148484cdf2ea543636ade49f3100d3be7489d7a4a993776e60b317d6d9b2f603"].into(), 18592931271096, CRU18),
        (hex!["2a6a8526f009e19120e40e7b31991908f19bf9a7e97f0544459ece922fdff44a"].into(), 2995544135100000, CRU18),
        (hex!["ce448621924bd3b65e3943cec822a3d7ea490c821185165299781715cbe9486a"].into(), 17000000000000000, CRU18),
        (hex!["3a39979329bf98de6efd85af9f25e738792140e095ef872c90bd10fdca9a5653"].into(), 1416100000000000, CRU18),
        (hex!["cc8b68461b9ee769ed7869559b8b100681ef8fdefabd37acf4c1eef346295237"].into(), 1610768669723130, CRU18),
        (hex!["4477c98e9787843e952aaeeba0b72dab89cbfb123e8006082024076a1387e174"].into(), 708050000000000, CRU18),
        (hex!["e653f18d61232e611588c5c492d4a65cf5074014386c31075da6f81835eb8c22"].into(), 89894741140000, CRU18),
        (hex!["707381bc23de93aa45fbf2fe35ee938b62932ef87fe16564aa410f570eaab075"].into(), 1268922907644570, CRU18),
        (hex!["360d393eee82f4c015b4a26e38b36efd12142be5bd8093004974fbd6f5d4854d"].into(), 725575956187587, CRU18),
        (hex!["2a6a8526f009e19120e40e7b31991908f19bf9a7e97f0544459ece922fdff44a"].into(), 7916104669221110, CRU18),
        (hex!["287479924922354b29d58a048af6251cc51230ee7cc00681fc991d98cde89828"].into(), 42500000000000000, CRU18),
        (hex!["34070b64189960c0fb0b181cb6735314e3f8662be4199221508376596ce4d926"].into(), 17000000000000000, CRU18),
        (hex!["740d2e9ae5aa0a83b6986dcc860202abc09bbf8d32ac0a0d8bd621c8e3403344"].into(), 1878630533655000, CRU18),
        (hex!["642c56cf902ba4d74c8742441fe66f5b2a0f5fc27cf8c12b8b4150f6bb01ef42"].into(), 349170566760000, CRU18),
        (hex!["2a6a8526f009e19120e40e7b31991908f19bf9a7e97f0544459ece922fdff44a"].into(), 13459289651576400, CRU18),
        (hex!["848d8eb2791621a8b6dec44bc81bc3479ef9ebe5bfeb0085a32931275187b040"].into(), 4250000000000000, CRU18),
        (hex!["0c62872d71c994ec07ed1c7a860ef5ea394b94497a41aa0bf6e50d50e6cf2814"].into(), 39999300000000000, CRU18),
        (hex!["368539ec0d760b4a476dc7ff62f6dce64e5e73818614863ea976ee0f3f9c4417"].into(), 33444789202000, CRU18),
        (hex!["a6d3f82ab40e19ee0670f42dde5e82175a3695976ab77b276681ee83f7429e03"].into(), 4250000000000000, CRU18),
        (hex!["463a3a4cfca3fabb653cbb6c6a6843c7fbe31f8100cd0b9c80db2e2a838f594e"].into(), 1045146679188660, CRU18),
        (hex!["6e3cc5498ec43be4c44b4872c79ece603ffbb33a7ddd7385ae99db4b0052b611"].into(), 17000000000000000, CRU18),
        (hex!["9093b00938937536b3ee4aa86985f8eb3c3137a39c722e341ba99a9fd9f4cd01"].into(), 17000000000000000, CRU18),
        (hex!["7229a956b52e7d2c70e9603e1fc1945c39e24e2ae24ed22d0b987d5c0ce6f464"].into(), 116620000000000, CRU18),
        (hex!["22837c9b9b8ddd3801e5781c06ee7954876d4221a1fceb35352b7c77f9d1c37c"].into(), 85000000000000000, CRU18),
        (hex!["cc8b68461b9ee769ed7869559b8b100681ef8fdefabd37acf4c1eef346295237"].into(), 1756296582684350, CRU18),
        (hex!["e653f18d61232e611588c5c492d4a65cf5074014386c31075da6f81835eb8c22"].into(), 29360259910000, CRU18),
        (hex!["66085c61c155fa371d81a32b4ea65e17d5a805e769f211d8bfc5277bd5ea1f01"].into(), 2975000000000000, CRU18),
        (hex!["ce21be0ba74408f33cdb2335ed73dfb8bb422fdd5f3037a6a96011a55b067d7f"].into(), 413258439907881, CRU18),
        (hex!["f4d95d4c5c0131969148d3a16b3d95ab3d051771d971a1955d7e745b0a3a4f16"].into(), 87579937659056, CRU18),
        (hex!["a8701aba8382f149a0ae54e56674c2bb21030d7a9370960583088b0c67203840"].into(), 708050000000000, CRU18),
        (hex!["d643a98cac26afd4fea4b3c0499b57e8159ce01a33cb06f9c23a4966c6845a3f"].into(), 211166574700000, CRU18),
        (hex!["e653f18d61232e611588c5c492d4a65cf5074014386c31075da6f81835eb8c22"].into(), 30537939960000, CRU18),
        (hex!["088e545114020f8ca61068bd029a695b667d4ba6adb3b626e57f13abfe7b5f1a"].into(), 153960032740000, CRU18),
        (hex!["e4041661a04b2a76da6c3f801fa5ca6b38ee55322a7329811dd339cf00b1fc5e"].into(), 465639813017051, CRU18),
        (hex!["e208e48dcbfcfe4d16532d242f4c1bc73573dff6428136f83f2fdcd68ace0526"].into(), 1874427742700000, CRU18),
        (hex!["764c421c2695393cb0aced6b89c22612669938c6442ab2dbd341e672dc89ad77"].into(), 708050000000000, CRU18),
        (hex!["e653f18d61232e611588c5c492d4a65cf5074014386c31075da6f81835eb8c22"].into(), 55630615040000, CRU18),
        (hex!["90f5c4a4ff2e23edd210a17425aa0e07107eb542fc0d04c21ed287bc2ceee743"].into(), 42500000000000000, CRU18),
        (hex!["0870134f05aaf4fe0743f826558ca2c6267e3d9cdea112489e91f0a750a8180b"].into(), 17000000000000000, CRU18),
        (hex!["cc8b68461b9ee769ed7869559b8b100681ef8fdefabd37acf4c1eef346295237"].into(), 1142378721400000, CRU18),
        (hex!["a65fad8d26950bd488aca53c8a8f7498a41a41f7f081e5d91e6b94f2b760de2c"].into(), 34000000000000000, CRU18),
        (hex!["00b26166231f3b040ac3903881b2268805c745cda08aed1b0d3fcfa3e91eea67"].into(), 97383478200000, CRU18),
        (hex!["82614e7346152fd97aa1201330eb95fb998c4930257f3d7955e255fc8215db50"].into(), 950368445490603, CRU18),
        (hex!["304ea081f601b93539392d547418ad5740dde5b381dbfbe184962e3b529ce264"].into(), 431045481800847, CRU18),
        (hex!["a6ac2b068c5e98e4bb911c31d29d332a20c8e32e2ca13b6d6454c54262f14507"].into(), 53158211683000, CRU18),
        (hex!["7ec8c764258fd07c95c1bfa1e00a94506b525e6ee35bfd336ca95f11d673174c"].into(), 17000000000000000, CRU18),
        (hex!["f603fb5d49bc3004c32710063454bb15779d06ad836e40393670be432231d957"].into(), 17000000000000000, CRU18),
        (hex!["12f5cfa1604e670316cab7eb0bc4741e9a84478283ff8c522196832e34f67b73"].into(), 210703720574912, CRU18),
        (hex!["cc8b68461b9ee769ed7869559b8b100681ef8fdefabd37acf4c1eef346295237"].into(), 1849454220595500, CRU18),
        (hex!["2ce484c619dced9c7c40ea2bbc9ab47c5e34f3525af06cf63cbaeac7c7115c46"].into(), 12750000000000000, CRU18),
        (hex!["cc8b68461b9ee769ed7869559b8b100681ef8fdefabd37acf4c1eef346295237"].into(), 2505215221376160, CRU18),
        (hex!["ac1582ce03e44347f849d6b6df380e9c9ba99e6686cde9bcb22756544ac3d676"].into(), 71812786845560, CRU18),
        (hex!["368539ec0d760b4a476dc7ff62f6dce64e5e73818614863ea976ee0f3f9c4417"].into(), 1434419615000, CRU18),
        (hex!["1af5832228285d20f9f67c41c0e38fa520ce076d63094f88b51fac2d0981cf14"].into(), 4308038737639080, CRU18),
        (hex!["bc13c9a902a524609f064014695f2b6548a17d7e8bb12a834220559bc38bbc5d"].into(), 12750000000000000, CRU18),
        (hex!["e208e48dcbfcfe4d16532d242f4c1bc73573dff6428136f83f2fdcd68ace0526"].into(), 107124099884660, CRU18),
        (hex!["faa063447b8254a1474ebcd6b61e7856ed8a2e281d6cc94bdc9ac5e4d66b606c"].into(), 514087111840000, CRU18),
        (hex!["fcb4c9a4eb92b63c54d1865c0c025e103f521885043bdf8d7c3e6ad8647c320b"].into(), 8978633859740530, CRU18),
        (hex!["42e97a0390fce748b601c9a382b9c25ca4b79fc2fa5382c311bfcd9384b54122"].into(), 28585757675000, CRU18),
        (hex!["cc8b68461b9ee769ed7869559b8b100681ef8fdefabd37acf4c1eef346295237"].into(), 406162776590000, CRU18),
        (hex!["ec19f11954855215e6598a3c6f6fb9cac09f10128e9620363cd39c215259067b"].into(), 8500000000000000, CRU18),
        (hex!["cc8b68461b9ee769ed7869559b8b100681ef8fdefabd37acf4c1eef346295237"].into(), 1521563016472440, CRU18),
        (hex!["92cb7979371ffe6111cd40daf1661fb758e42cb561385825e46bff35cb041431"].into(), 16361530779229300, CRU18),
        (hex!["ee98c02253479726b456f3f7c5a9014d0198f53717de881c15f85becd706313c"].into(), 127500000000000000, CRU18),
        (hex!["5819c98f3e396d8c4443a05918202adda8b5626d8dd231d95de57aabd7400d58"].into(), 1428722285728810, CRU18),
        (hex!["209b2f781722fb511a129dfd1b88018e5b79c502b37e5ee24cc07d43a626c730"].into(), 1275000000000000, CRU18),
        (hex!["cc8b68461b9ee769ed7869559b8b100681ef8fdefabd37acf4c1eef346295237"].into(), 742342388100000, CRU18),
        (hex!["cc8b68461b9ee769ed7869559b8b100681ef8fdefabd37acf4c1eef346295237"].into(), 3042752510732370, CRU18),
        (hex!["8c5244923b757cec4bab4002ec0b5386e079a3c7ed8d147f3742dd5df8ade26e"].into(), 21250000000000000, CRU18),
        (hex!["cc8b68461b9ee769ed7869559b8b100681ef8fdefabd37acf4c1eef346295237"].into(), 3640072052188530, CRU18),
        (hex!["18fdd7b6f9ffb16214e486d871df13793d07236bbe6ba8f4411a9c0509dc983d"].into(), 12907902411506, CRU18),
        (hex!["aaed06223584d7969543f089300133de402f6c93d26704de065bf50f605b4308"].into(), 42500000000000000, CRU18),
        (hex!["2a6a8526f009e19120e40e7b31991908f19bf9a7e97f0544459ece922fdff44a"].into(), 6894572979449980, CRU18),
        (hex!["248886cacfedd6809488fd8a7be9e90191edf193ad908b6087c3f220d6cf6d3d"].into(), 2777566219400000, CRU18),
        (hex!["368539ec0d760b4a476dc7ff62f6dce64e5e73818614863ea976ee0f3f9c4417"].into(), 111427987891795, CRU18),
        (hex!["3e6a0ffa024569d09176419f2ed1bb35bd763e6ff03f1e977c4f7fbe58317f63"].into(), 163765263143509, CRU18),
        (hex!["5eb5452ddee682be165f2556d6a962c9d12c867fc92a9ac7720302b5f4b65f40"].into(), 774013001314448, CRU18),
        (hex!["20a37aba6671466442d7b45107f4c79c2cc436d46a2848cc67408ebd6c29a45e"].into(), 17000000000000000, CRU18),
        (hex!["4472528d682e6fbd6ab29f28dea1d82a206e1a793059fa1ec36bd385b9e25815"].into(), 5076529510279, CRU18),
        (hex!["00b26166231f3b040ac3903881b2268805c745cda08aed1b0d3fcfa3e91eea67"].into(), 85228003344000, CRU18),
        (hex!["368539ec0d760b4a476dc7ff62f6dce64e5e73818614863ea976ee0f3f9c4417"].into(), 37084815653198, CRU18),
        (hex!["368539ec0d760b4a476dc7ff62f6dce64e5e73818614863ea976ee0f3f9c4417"].into(), 17303684562998, CRU18),
        (hex!["cc8b68461b9ee769ed7869559b8b100681ef8fdefabd37acf4c1eef346295237"].into(), 2291851035637420, CRU18),
        (hex!["cc8b68461b9ee769ed7869559b8b100681ef8fdefabd37acf4c1eef346295237"].into(), 740735486375091, CRU18),
        (hex!["688b9d1b628bd16969a6a44576dc7341c3c9554732ee5a19fb2a87751c1ccf64"].into(), 64401621849000, CRU18),
        (hex!["ba8310f1f784d580ed36777b18507fbb8c6a516983269f71949aeadcb614ed77"].into(), 402266892129021, CRU18),
        (hex!["cc8b68461b9ee769ed7869559b8b100681ef8fdefabd37acf4c1eef346295237"].into(), 921205413700000, CRU18),
        (hex!["56c6968d896ef9a5c873c4c2bb579d8035ba5b09457662f67a77e772b9254613"].into(), 47864983360405, CRU18),
        (hex!["7cf287d43b2f2535ab2d91aea1c48464f2bb6efba0a7c926d6ab4d76a6a21b4e"].into(), 1181068859289030, CRU18),
        (hex!["90e73e403c3593a32f97907121ed88f2b8cd913421fe0fc471053532e1707012"].into(), 8500000000000000, CRU18),
        (hex!["cc8b68461b9ee769ed7869559b8b100681ef8fdefabd37acf4c1eef346295237"].into(), 2069957527807480, CRU18),
        (hex!["baa3b29b0b8d2b517cd108929af8735abeaffea190d71d61a101f0e71000a374"].into(), 25500000000000000, CRU18),
        (hex!["3c36667e4442caf6f70e19984f1eab388d2d4673fe9166caf12e0174e042884c"].into(), 42500000000000000, CRU18),
        (hex!["a831b2f842efb1803901f2c4a0af083bc54df024be001c0e870b8248a9194277"].into(), 17000000000000000, CRU18),
        (hex!["e024724af1d9e022c3e1be7d49c882724bbf33d348978117b31cd85f8b6e4334"].into(), 280000000000000, CRU18),
        (hex!["18fdd7b6f9ffb16214e486d871df13793d07236bbe6ba8f4411a9c0509dc983d"].into(), 11215115253227, CRU18),
        (hex!["40bca9244cfc7b126180c07f465f38cfd5d7e30d9424c8cb5941d529b718361a"].into(), 68000000000000000, CRU18),
        (hex!["64108cdb2275d42fe6ef0bbf8d4ec91e973f102cd3cb897418aa5887ffde7b39"].into(), 6375000000000000, CRU18),
        (hex!["088e545114020f8ca61068bd029a695b667d4ba6adb3b626e57f13abfe7b5f1a"].into(), 115304918831000, CRU18),
        (hex!["088e545114020f8ca61068bd029a695b667d4ba6adb3b626e57f13abfe7b5f1a"].into(), 5225230628893, CRU18),
        (hex!["cc8b68461b9ee769ed7869559b8b100681ef8fdefabd37acf4c1eef346295237"].into(), 3016415480636140, CRU18),
        (hex!["18fdd7b6f9ffb16214e486d871df13793d07236bbe6ba8f4411a9c0509dc983d"].into(), 8941150114883, CRU18),
        (hex!["ac1582ce03e44347f849d6b6df380e9c9ba99e6686cde9bcb22756544ac3d676"].into(), 85987165458278, CRU18),
        (hex!["d2bc52d1d41596559bb6a1573181ed6103f810091abff49d51a71d26b0e5f857"].into(), 723350379486956, CRU18),
        (hex!["d0b193a5da0beb08dfaa310a64903be917bba0b08bba9f1de002710e3d8eb610"].into(), 708050000000000, CRU18),
        (hex!["7cc4e8e83325b4d316990362a0a56c655eeca65bd9034d4839fc3957634cbd42"].into(), 34000000000000000, CRU18),
        (hex!["1a585f4dea236cbf9f78ed9733bf033a81fc986c2d7f6865f56c37ea4ffe3f7f"].into(), 17507611878759800, CRU18),
        (hex!["74ed3187966c99c0e0196e76a75a06fe52f52a91069a9f73e6d4243c5f45a553"].into(), 23761820593414, CRU18),
        (hex!["368539ec0d760b4a476dc7ff62f6dce64e5e73818614863ea976ee0f3f9c4417"].into(), 21033644012817, CRU18),
        (hex!["78c7e8044d4e47dc2a21744253a2e1e49d5a6bdfd01b0e4af49d54b68cb30871"].into(), 25500000000000000, CRU18),
        (hex!["be9936438ccfa98d0eb084b4a8599a4a7d5d0224501c12decf6bdd9df38ca034"].into(), 4900000000000000, CRU18),
        (hex!["cc8b68461b9ee769ed7869559b8b100681ef8fdefabd37acf4c1eef346295237"].into(), 296857706257382, CRU18),
        (hex!["80ca5daabae19a57d65d62369b56b02af21735a1214a11542f7e0922a4bd270c"].into(), 12750000000000000, CRU18),
        (hex!["cc8b68461b9ee769ed7869559b8b100681ef8fdefabd37acf4c1eef346295237"].into(), 6637145149826090, CRU18),
        (hex!["a820ecc974f0cd4e82538b9e426b6148bd309e26782c82f18d25094e146e7677"].into(), 68259453583058, CRU18),
        (hex!["ba95da1ecdd2e08f9a49c68a5febb620ca8d2869befe0bd1ad27af30847f765a"].into(), 34000000000000000, CRU18),
        (hex!["cc8b68461b9ee769ed7869559b8b100681ef8fdefabd37acf4c1eef346295237"].into(), 1717041869799490, CRU18),
        (hex!["fca58ce3d54c5faf729b8bfe3b95f2481f2ae095ea9a28f415b929ae985b6679"].into(), 14000000000000000, CRU18),
        (hex!["02fb7cfd175700fc9a41b2b35fba08e015a7d625c49fff0cf9c457e79ba9e77e"].into(), 24151504526170, CRU18)
    ];

    // Constants
    const ENDOWMENT: u128 = 10 * CRUS;
    const STASH: u128 = 10 * CRUS;

    GenesisConfig {
        pallet_sudo: Some(SudoConfig {
            key: endowed_accounts[0].clone(),
        }),
        frame_system: Some(SystemConfig {
            code: wasm_binary.to_vec(),
            changes_trie_config: Default::default(),
        }),
        balances_Instance1: Some(BalancesConfig {
            balances: endowed_accounts
                .iter()
                .cloned()
                .map(|k| (k, ENDOWMENT))
                .collect(),
        }),
        pallet_indices: Some(IndicesConfig {
            indices: vec![],
        }),
        pallet_session: Some(SessionConfig {
            keys: initial_authorities.iter().map(|x| (
                x.0.clone(),
                x.0.clone(),
                session_keys(x.2.clone(), x.3.clone(), x.4.clone(), x.5.clone()),
            )).collect::<Vec<_>>(),
        }),
        staking: Some(StakingConfig {
            validator_count: 1,
            minimum_validator_count: 1,
            stakers: initial_authorities
                .iter()
                .map(|x| (x.0.clone(), x.1.clone(), STASH, StakerStatus::Validator))
                .collect(),
            invulnerables: initial_authorities.iter().map(|x| x.0.clone()).collect(),
            force_era: Forcing::NotForcing,
            slash_reward_fraction: Perbill::from_percent(10),
            ..Default::default()
        }),
        market: Some(Default::default()),
        pallet_babe: Some(Default::default()),
        pallet_grandpa: Some(Default::default()),
        pallet_im_online: Some(Default::default()),
        pallet_authority_discovery: Some(AuthorityDiscoveryConfig {
            keys: vec![]
        }),
        swork: Some(SworkConfig {
            init_codes: vec![]
        }),
        locks: Some(LocksConfig {
            genesis_locks: initial_locks
        }),
        pallet_treasury: Some(Default::default()),
    }
}
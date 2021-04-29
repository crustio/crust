// Copyright (C) 2019-2021 Crust Network Technologies Ltd.
// This file is part of Crust.

use hex_literal::hex;
use sp_core::{Pair, Public, sr25519, crypto::UncheckedInto};
use crust_runtime::{
    AuthorityDiscoveryId, BalancesConfig, GenesisConfig, ImOnlineId,
    AuthorityDiscoveryConfig, SessionConfig, SessionKeys, StakerStatus,
    StakingConfig, IndicesConfig, SystemConfig, SworkConfig, SudoConfig,
    ElectionsConfig, CouncilConfig, TechnicalCommitteeConfig, DemocracyConfig,
    WASM_BINARY
};
use cstrml_staking::Forcing;
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_consensus_babe::AuthorityId as BabeId;
use primitives::{constants::currency::CRUS, *};
use sc_service::ChainType;
use sp_runtime::{traits::{Verify, IdentifyAccount}, Perbill};

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
	let num_endowed_accounts = endowed_accounts.len();
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
        balances_Instance2: Some(Default::default()),
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
        pallet_babe: Some(Default::default()),
        pallet_grandpa: Some(Default::default()),
        pallet_im_online: Some(Default::default()),
        pallet_authority_discovery: Some(AuthorityDiscoveryConfig {
            keys: vec![]
        }),
        swork: Some(SworkConfig {
            init_codes: vec![]
        }),
        pallet_collective_Instance1: Some(CouncilConfig::default()),
        pallet_treasury: Some(Default::default()),
        pallet_elections_phragmen: Some(ElectionsConfig {
			members: endowed_accounts.iter()
						.take((num_endowed_accounts + 1) / 2)
						.cloned()
						.map(|member| (member, STASH))
						.collect(),
		}),
        pallet_collective_Instance2: Some(TechnicalCommitteeConfig {
            members: endowed_accounts.iter()
                .take((num_endowed_accounts + 1) / 2)
                .cloned()
                .collect(),
            phantom: Default::default(),
        }),
        pallet_democracy: Some(DemocracyConfig::default()),
        pallet_membership_Instance1: Some(Default::default()),
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
    let num_endowed_accounts = endowed_accounts.len();

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
        balances_Instance2: Some(Default::default()),
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
        pallet_babe: Some(Default::default()),
        pallet_grandpa: Some(Default::default()),
        pallet_im_online: Some(Default::default()),
        pallet_authority_discovery: Some(AuthorityDiscoveryConfig {
            keys: vec![]
        }),
        swork: Some(SworkConfig {
            init_codes: vec![]
        }),
        pallet_collective_Instance1: Some(CouncilConfig::default()),
        pallet_treasury: Some(Default::default()),
        pallet_elections_phragmen: Some(ElectionsConfig {
			members: endowed_accounts.iter()
						.take((num_endowed_accounts + 1) / 2)
						.cloned()
						.map(|member| (member, STASH))
						.collect(),
		}),
        pallet_collective_Instance2: Some(TechnicalCommitteeConfig {
            members: endowed_accounts.iter()
                .take((num_endowed_accounts + 1) / 2)
                .cloned()
                .collect(),
            phantom: Default::default(),
        }),
        pallet_democracy: Some(DemocracyConfig::default()),
        pallet_membership_Instance1: Some(Default::default()),
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
    let num_endowed_accounts = endowed_accounts.len();

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
    const ENDOWMENT: u128 = 200 * CRUS;
    const STASH: u128 = 100 * CRUS;

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
        balances_Instance2: Some(Default::default()),
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
        pallet_babe: Some(Default::default()),
        pallet_grandpa: Some(Default::default()),
        pallet_im_online: Some(Default::default()),
        pallet_authority_discovery: Some(AuthorityDiscoveryConfig {
            keys: vec![]
        }),
        swork: Some(SworkConfig {
            init_codes: vec![]
        }),
        pallet_collective_Instance1: Some(CouncilConfig::default()),
        pallet_treasury: Some(Default::default()),
        pallet_elections_phragmen: Some(ElectionsConfig {
			members: endowed_accounts.iter()
						.take((num_endowed_accounts + 1) / 2)
						.cloned()
						.map(|member| (member, STASH))
						.collect(),
		}),
        pallet_collective_Instance2: Some(TechnicalCommitteeConfig {
            members: endowed_accounts.iter()
                .take((num_endowed_accounts + 1) / 2)
                .cloned()
                .collect(),
            phantom: Default::default(),
        }),
        pallet_democracy: Some(DemocracyConfig::default()),
        pallet_membership_Instance1: Some(Default::default()),
    }
}
use hex_literal::hex;
use sp_core::{Pair, Public, sr25519, crypto::UncheckedInto};
use crust_runtime::{
    AuthorityDiscoveryId, BalancesConfig, GenesisConfig, ImOnlineId,
    AuthorityDiscoveryConfig, SessionConfig, SessionKeys, StakerStatus,
    StakingConfig, IndicesConfig, SystemConfig, TeeConfig, SudoConfig,
    WASM_BINARY
};
use cstrml_staking::Forcing;
use cstrml_tee::{WorkReport, Identity};
use grandpa_primitives::AuthorityId as GrandpaId;
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

/// Crust rocky(aka. internal testnet) config
pub fn rocky_config() -> Result<CrustChainSpec, String> {
    CrustChainSpec::from_json_bytes(&include_bytes!("../res/rocky.json")[..])
}

/// Crust rocky staging config
pub fn rocky_staging_config() -> CrustChainSpec {
    CrustChainSpec::from_genesis(
        "Crust Rocky",
        "crust_rocky",
        ChainType::Live,
        rocky_staging_testnet_config_genesis,
        vec![],
        None,
        Some(DEFAULT_PROTOCOL_ID),
        None,
        Default::default()
    )
}

/// Crust development config (single validator Alice)
pub fn development_config() -> CrustChainSpec {
    CrustChainSpec::from_genesis(
        "Development",
        "dev",
        ChainType::Development,
        || testnet_genesis(
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
    )
}

/// Crust local testnet config (multi-validator Alice + Bob)
pub fn local_testnet_config() -> CrustChainSpec {
    CrustChainSpec::from_genesis(
        "Local Testnet",
        "local_testnet",
        ChainType::Local,
        || testnet_genesis(
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
    )
}

/// The genesis spec of crust rocky test network
fn rocky_staging_testnet_config_genesis() -> GenesisConfig {
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
    const WORKLOAD: u64 = 1073741824;

    GenesisConfig {
        sudo: Some(SudoConfig {
            key: endowed_accounts[0].clone(),
        }),
        system: Some(SystemConfig {
            code: WASM_BINARY.to_vec(),
            changes_trie_config: Default::default(),
        }),
        balances: Some(BalancesConfig {
            balances: endowed_accounts
                .iter()
                .cloned()
                .map(|k| (k, ENDOWMENT))
                .collect(),
        }),
        indices: Some(IndicesConfig {
            indices: vec![],
        }),
        session: Some(SessionConfig {
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
        babe: Some(Default::default()),
        grandpa: Some(Default::default()),
        im_online: Some(Default::default()),
        authority_discovery: Some(AuthorityDiscoveryConfig {
            keys: vec![]
        }),
        tee: Some(TeeConfig {
            code: vec![],
            current_report_slot: 0,
            identities: endowed_accounts
                .iter()
                .map(|x| (
                    x.clone(),
                    Identity {
                        ias_sig: vec![],
                        ias_cert: vec![],
                        account_id: x.clone(),
                        isv_body: vec![],
                        pub_key: vec![],
                        code: vec![],
                        sig: vec![]
                    }
                ))
                .collect(),
            work_reports: endowed_accounts
                .iter()
                .map(|x| (
                    x.clone(),
                    WorkReport {
                        pub_key: vec![],
                        block_number: 0,
                        block_hash: vec![],
                        files: vec![],
                        reserved: WORKLOAD,
                        sig: vec![],
                        used: 0
                    },
                ))
                .collect(),
        }),
    }
}

/// The genesis spec of crust dev/local test network
fn testnet_genesis(
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
    const WORKLOAD: u64 = 40_000 * 1000_000;

    GenesisConfig {
        sudo: Some(SudoConfig {
            key: endowed_accounts[0].clone(),
        }),
        system: Some(SystemConfig {
            code: WASM_BINARY.to_vec(),
            changes_trie_config: Default::default(),
        }),
        balances: Some(BalancesConfig {
            balances: endowed_accounts
                .iter()
                .cloned()
                .map(|k| (k, ENDOWMENT))
                .collect(),
        }),
        indices: Some(IndicesConfig {
            indices: vec![],
        }),
        session: Some(SessionConfig {
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
        babe: Some(Default::default()),
        grandpa: Some(Default::default()),
        im_online: Some(Default::default()),
        authority_discovery: Some(AuthorityDiscoveryConfig { 
            keys: vec![] 
        }),
        tee: Some(TeeConfig {
            code: vec![],
            current_report_slot: 0,
            identities: endowed_accounts
                .iter()
                .map(|x| (x.clone(), Default::default()))
                .collect(),
            work_reports: endowed_accounts
                .iter()
                .map(|x| {
                    (
                        x.clone(),
                        WorkReport {
                            pub_key: vec![],
                            block_number: 0,
                            block_hash: vec![],
                            files: vec![],
                            reserved: WORKLOAD,
                            sig: vec![],
                            used: 0
                        },
                    )
                })
                .collect(),
        })
    }
}
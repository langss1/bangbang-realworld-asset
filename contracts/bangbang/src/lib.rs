//! ╔══════════════════════════════════════════════════════════════════════╗
//! ║           Bang Bang — Real World Asset (RWA) Protocol              ║
//! ║       Stellar / Soroban Smart Contract  •  soroban-sdk v21         ║
//! ╚══════════════════════════════════════════════════════════════════════╝
//!
//! Purpose:
//!   Record, validate, and manage physical asset portfolios on-chain.
//!   Supported asset types include: Gold, Land, Property, Vehicle, etc.
//!
//! Storage layout (instance storage):
//!   ASSET_COUNT  →  u64                      — monotonic ID counter
//!   ASSETS       →  Vec<Asset>               — all registered assets
//!
//! Functions:
//!   • create_asset(asset_type, description, value_in_usd) -> u64
//!   • get_assets()                                        -> Vec<Asset>
//!   • delete_asset(id)                                    -> bool

#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype,
    symbol_short, vec, Env, String, Symbol, Vec,
};

// ─── Storage Key Symbols ──────────────────────────────────────────────────────

/// Key that stores the next available asset ID (u64).
const ASSET_COUNT: Symbol = symbol_short!("ASST_CNT");

/// Key that stores the Vec<Asset> list.
const ASSETS: Symbol = symbol_short!("ASSETS");

// ─── Data Types ───────────────────────────────────────────────────────────────

/// Represents a single Real-World Asset entry on chain.
///
/// Fields:
/// - `id`            — Unique, auto-incrementing identifier.
/// - `asset_type`    — Category string: "Gold", "Land", "Property", "Vehicle", …
/// - `description`   — Free-form textual description of the asset.
/// - `value_in_usd`  — Appraised USD value (integer, no decimals).
#[contracttype]
#[derive(Clone)]
pub struct Asset {
    pub id: u64,
    pub asset_type: String,
    pub description: String,
    pub value_in_usd: u64,
}

// ─── Contract ─────────────────────────────────────────────────────────────────

#[contract]
pub struct AssetContract;

#[contractimpl]
impl AssetContract {
    // ── CREATE ────────────────────────────────────────────────────────────────

    /// Register a new physical asset on-chain.
    ///
    /// # Arguments
    /// * `asset_type`    — Category of the asset (e.g. "Gold", "Land").
    /// * `description`   — Human-readable description.
    /// * `value_in_usd`  — Appraised USD value.
    ///
    /// # Returns
    /// The newly assigned asset `id`.
    pub fn create_asset(
        env: Env,
        asset_type: String,
        description: String,
        value_in_usd: u64,
    ) -> u64 {
        // Increment the global ID counter
        let mut count: u64 = env
            .storage()
            .instance()
            .get(&ASSET_COUNT)
            .unwrap_or(0u64);
        count += 1;

        // Build the new asset struct
        let new_asset = Asset {
            id: count,
            asset_type,
            description,
            value_in_usd,
        };

        // Load existing list (or start with empty vec), push new asset, save
        let mut assets: Vec<Asset> = env
            .storage()
            .instance()
            .get(&ASSETS)
            .unwrap_or_else(|| vec![&env]);

        assets.push_back(new_asset);

        env.storage().instance().set(&ASSETS, &assets);
        env.storage().instance().set(&ASSET_COUNT, &count);

        // Emit a lightweight event so indexers / clients can track additions
        env.events().publish(
            (symbol_short!("ASSET"), symbol_short!("created")),
            count,
        );

        count
    }

    // ── READ ──────────────────────────────────────────────────────────────────

    /// Retrieve all registered assets from on-chain storage.
    ///
    /// # Returns
    /// A `Vec<Asset>` (may be empty if no assets have been registered yet).
    pub fn get_assets(env: Env) -> Vec<Asset> {
        env.storage()
            .instance()
            .get(&ASSETS)
            .unwrap_or_else(|| vec![&env])
    }

    // ── DELETE ────────────────────────────────────────────────────────────────

    /// Remove an asset by its ID.
    ///
    /// Iterates the stored list, filters out the matching entry, and writes
    /// the updated list back to instance storage.
    ///
    /// # Arguments
    /// * `id` — The unique ID of the asset to remove.
    ///
    /// # Returns
    /// `true` if an asset was found and removed, `false` if the ID did not exist.
    pub fn delete_asset(env: Env, id: u64) -> bool {
        let assets: Vec<Asset> = env
            .storage()
            .instance()
            .get(&ASSETS)
            .unwrap_or_else(|| vec![&env]);

        // Collect all assets that do NOT match the target ID
        let mut updated: Vec<Asset> = vec![&env];
        let mut found = false;

        for i in 0..assets.len() {
            let asset = assets.get(i).unwrap();
            if asset.id == id {
                found = true; // mark as found, skip (do not push)
            } else {
                updated.push_back(asset);
            }
        }

        if found {
            env.storage().instance().set(&ASSETS, &updated);

            // Emit deletion event
            env.events().publish(
                (symbol_short!("ASSET"), symbol_short!("deleted")),
                id,
            );
        }

        found
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{Env, String};

    #[test]
    fn test_create_and_get() {
        let env = Env::default();
        let contract_id = env.register_contract(None, AssetContract);
        let client = AssetContractClient::new(&env, &contract_id);

        // Create two assets
        let id1 = client.create_asset(
            &String::from_str(&env, "Gold"),
            &String::from_str(&env, "24K gold bar, 1 kg"),
            &60_000u64,
        );
        let id2 = client.create_asset(
            &String::from_str(&env, "Land"),
            &String::from_str(&env, "2 hectare agricultural plot in Java"),
            &150_000u64,
        );

        assert_eq!(id1, 1);
        assert_eq!(id2, 2);

        let all = client.get_assets();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_delete_asset() {
        let env = Env::default();
        let contract_id = env.register_contract(None, AssetContract);
        let client = AssetContractClient::new(&env, &contract_id);

        client.create_asset(
            &String::from_str(&env, "Vehicle"),
            &String::from_str(&env, "Toyota Land Cruiser 2023"),
            &80_000u64,
        );
        client.create_asset(
            &String::from_str(&env, "Property"),
            &String::from_str(&env, "3-bedroom house in Bandung"),
            &200_000u64,
        );

        // Delete asset with id = 1
        let removed = client.delete_asset(&1u64);
        assert!(removed);

        let remaining = client.get_assets();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining.get(0).unwrap().id, 2);
    }

    #[test]
    fn test_delete_nonexistent() {
        let env = Env::default();
        let contract_id = env.register_contract(None, AssetContract);
        let client = AssetContractClient::new(&env, &contract_id);

        // Deleting from an empty store should return false
        let removed = client.delete_asset(&99u64);
        assert!(!removed);
    }
}

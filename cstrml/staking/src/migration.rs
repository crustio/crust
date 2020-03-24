//! Storage migrations for cstrml-staking.

/// Indicator of a version of a storage layout.
pub type VersionNumber = u32;

// the current expected version of the storage
pub const CURRENT_VERSION: VersionNumber = 1;

#[cfg(any(test, feature = "migrate"))]
mod inner {
    use super::{VersionNumber, CURRENT_VERSION};
    use crate::{BalanceOf, IndividualExposure, Module, Store, Trait};
    use frame_support::{StorageLinkedMap, StorageValue};
    use sp_runtime::traits::Zero;
    use sp_std::vec::Vec;

    // the minimum supported version of the migration logic.
    const MIN_SUPPORTED_VERSION: VersionNumber = 0;

    // migrate storage from v0 to v1.
    //
    // this upgrades the `Guarantors` linked_map value type from `Vec<T::AccountId>` to
    // `Option<Nominations<T::AccountId, BalanceOf<T>>`
    pub fn to_v1<T: Trait>(version: &mut VersionNumber) {
        if *version != 0 {
            return;
        }
        *version += 1;

        let now = <Module<T>>::current_era();
        let res =
            <Module<T> as Store>::Guarantors::translate::<T::AccountId, Vec<T::AccountId>, _, _>(
                |key| key,
                |targets| crate::Nominations {
                    targets: targets
                        .iter()
                        .map(|t| {
                            crate::IndividualExposure {
                                who: t.clone(),
                                // TODO: This is wrong, but we don't have migration, so we don't care 😈
                                value: Zero::zero(),
                            }
                        })
                        .collect::<Vec<IndividualExposure<T::AccountId, BalanceOf<T>>>>(),
                    submitted_in: now,
                    suppressed: false,
                },
            );

        if let Err(e) = res {
            frame_support::print("Encountered error in migration of Staking::Guarantors map.");
            if e.is_none() {
                frame_support::print("Staking::Guarantors map reinitialized");
            }
        }

        frame_support::print("Finished migrating Staking storage to v1.");
    }

    pub(super) fn perform_migrations<T: Trait>() {
        <Module<T> as Store>::StorageVersion::mutate(|version| {
            if *version < MIN_SUPPORTED_VERSION {
                frame_support::print(
                    "Cannot migrate staking storage because version is less than\
					minimum.",
                );
                frame_support::print(*version);
                return;
            }

            if *version == CURRENT_VERSION {
                return;
            }

            to_v1::<T>(version);
        });
    }
}

#[cfg(not(any(test, feature = "migrate")))]
mod inner {
    pub(super) fn perform_migrations<T>() {}
}

/// Perform all necessary storage migrations to get storage into the expected stsate for current
/// logic. No-op if fully upgraded.
pub(crate) fn perform_migrations<T: crate::Trait>() {
    inner::perform_migrations::<T>();
}

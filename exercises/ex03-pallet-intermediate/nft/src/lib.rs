#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/v3/runtime/frame>
pub use pallet::*;

#[cfg(test)]
mod tests;
pub mod types;

use frame_support::ensure;
use types::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{pallet_prelude::*, traits::{fungible::Inspect, tokens::AssetId}};
	use frame_system::pallet_prelude::{*, OriginFor};

	#[pallet::config]
	pub trait Config: frame_system::Config + scale_info::TypeInfo {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		#[pallet::constant]
		type MaxLength: Get<u32>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn unique_asset)]
	pub(super) type UniqueAsset<T: Config> =
		StorageMap<_, Blake2_128Concat, UniqueAssetId, UniqueAssetDetails<T, T::MaxLength>>;

	#[pallet::storage]
	#[pallet::getter(fn account)]
	/// The holdings of a specific account for a specific asset.
	pub(super) type Account<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		UniqueAssetId,
		Blake2_128Concat,
		T::AccountId,
		u128,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn nonce)]
	/// Nonce for id of the next created asset
	pub(super) type Nonce<T: Config> = StorageValue<_, UniqueAssetId, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// New unique asset created
		Created {
			creator: T::AccountId,
			asset_id: UniqueAssetId,
		},
		/// Some assets have been burned
		Burned {
			asset_id: UniqueAssetId,
			owner: T::AccountId,
			total_supply: u128,
		},
		/// Some assets have been transferred
		Transferred {
			asset_id: UniqueAssetId,
			from: T::AccountId,
			to: T::AccountId,
			amount: u128,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The asset ID is unknown
		UnknownAssetId,
		/// The signing account does not own any amount of this asset
		NotOwned,
		/// Supply must be positive
		NoSupply,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0)]
		pub fn mint(
			origin: OriginFor<T>,
			metadata: BoundedVec<u8, T::MaxLength>,
			supply: u128,
		) -> DispatchResult {
			let origin = ensure_signed(origin.clone())?;

			ensure!(supply > 1, Error::<T>::NoSupply);

			let id = Self::nonce();
			let asset_detail = UniqueAssetDetails::<T, T::MaxLength>::new(origin.clone(), metadata, supply);

			UniqueAsset::<T>::insert(id, asset_detail);
			Nonce::<T>::set(id.saturating_add(1));

			Account::<T>::mutate(id, origin.clone(), |balance| {
				*balance = balance.saturating_add(supply);
			});

			Self::deposit_event(Event::<T>::Created {
				creator: origin.clone(),
				asset_id: id,
			});

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn burn(origin: OriginFor<T>, asset_id: UniqueAssetId, amount: u128) -> DispatchResult {
			Ok(())
		}

		#[pallet::weight(0)]
		pub fn transfer(
			origin: OriginFor<T>,
			asset_id: UniqueAssetId,
			amount: u128,
			to: T::AccountId,
		) -> DispatchResult {
			let origin = ensure_signed(origin)?;

			ensure!(
				Self::unique_asset(asset_id).is_some(),
				Error::<T>::UnknownAssetId
			);

			Self::ensure_own_some(asset_id, origin.clone())?;

			let mut transferred_amount = 0;

			Account::<T>::mutate(asset_id, origin.clone(), |balance| {
				let old_balance = *balance;
				*balance = old_balance.saturating_sub(amount);
				transferred_amount = old_balance - *balance;
			});

			Account::<T>::mutate(asset_id, to.clone(), |balance| {
				*balance += transferred_amount;
			});

			Self::deposit_event(Event::<T>::Transferred {
				asset_id,
				from: origin.clone(),
				to: to.clone(),
				amount: transferred_amount,
			});

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		fn ensure_own_some(asset_id: UniqueAssetId, account: T::AccountId) -> Result<(), Error<T>> {
			let owned = Self::account(asset_id, account);

			ensure!(owned > 0, Error::<T>::NotOwned);

			Ok(())
		} 
	}
}

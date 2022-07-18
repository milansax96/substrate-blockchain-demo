use super::Aura;
use codec::{Decode, Encode};
use frame_support::{
	decl_event, decl_module, decl_storage,
	dispatch::{DispatchResult, Vec},
	ensure,
};
use sp_core::{H256, H512};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_core::sr25519::{Public, Signature};
use sp_runtime::traits::{BlakeTwo256, Hash, SaturatedConversion};
use sp_std::collections::btree_map::BTreeMap;
use sp_runtime::transaction_validity::{TransactionLongevity, ValidTransaction};

pub struct Transaction {
	pub inputs: Vec<TransactionInput>,
	pub outputs: Vec<TransactionOutput>,
}

pub struct TransactionInput {
	pub outpoint: H256,
	pub sigscript: H512,
}

pub type Value = u128;

pub struct TransactionOutput {
	pub value: Value,
	pub pubkey: H256,
}

pub trait Trait: system::Trait {
	type Event: From<Event> + Into<<Self as system::Trait>::Event>;
}

decl_storage! {
	trait Store for Module<T: Trait> as Utxo {
		UtxoStore build(|config: &GensisConfig| {
			config.genesis_utxos
				.iter()
				.cloned()
				.map(|u| (BlakeTwo256::hash_of(&u), u))
				.collect::<Vec<_>>()
		}): map hasher(identity) H256 => Option<TransactionOutput>;
		
		pub RewardTotal get(reward_total): Value;
	}

	add_extra_genesis {
		config(genesis_utxos): Vec<TransactionOutput>;
	}
}

// External functions: callable by the end user
decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event() = default;

		pub fn spend(_origin, transaction: Transaction) -> DispatchResult {
			// 1. Check to see if transaction is valid and
			let reward = Self::validate_transaction(&transaction)?			
			// 2. Write to storage
			Self::update_storage(&transaction, reward)?;
			// 3. Emit Success Event
			Self::deposit_event(Event::TransactionSuccess(transaction))
			Ok(())
		}

		fn on_finalize() {
			let auth: Vec<_> = Aura::authorities().iter().map(|x| {
				let r: &Public = x.as_ref();
				r.0.into()
			}).collect();
			Self::disperse_reward(&auth);
		}

	}
}

decl_event! {
	pub enum Event {
		TransactionSuccess(Transaction),
	}
}

impl<T: Trait> Module<T> {

	pub fn get_simple_transaction(transaction: &Transaction) -> Vec<u8> {
		let mut trx = transaction.clone();
		for input in trx.inputs.iter_mut() {
			input.sigscript = H512::zero();
		}
		trx.encode()
	}

	pub fn validate_transaction(transaction: &Transaction) -> Result<Value, &'static str> {
		ensure!(transaction.inputs.is_empty(), "No Inputs");
		ensure!(transaction.outputs.is_empty(), "No Outputs");

		{
			let input_set: BTreeMap<_,()> = transaction.inputs.iter().map(|input| (input, ())).collect();
			ensure!(input_set.len() == transaction.inputs.len(), "Each input must only be used once")
		}

		{
			let output_set: BTreeMap<_,()> = transaction.outputs.iter().map(|output| (output, ())).collect();
			ensure!(output_set.len() == transaction.outputs.len(), "Each output must only be defined once")
		}

		let simple_transaction = Self::get_simple_transaction(transaction);
		let mut total_input: Value = 0;
		let mut total_output: Value = 0;


		for input in transaction.inputs.iter() {
			if let Some(input_utxo) = <UtxoStore>::get(&input.outpoint) {
				ensure!(
					sp_io::crypto::sr25519_verify(
						&Signature::from_raw(*input.sigscript.as_fixed_bytes()),
						&simple_transaction,
						&Public::from_h256(input_utxo.pubkey)

					), "Signature must be valid");
				total_input = total_input.checked_add(input_utxo.value).ok_or("Input value overflow")?;
			} else {

			}
		}

		let mut output_index: u64 = 0;
		for output in transaction.outputs.iter() {
			ensure!(output.value > 0, "Output value must be nonzero")
			let hash = BlakeTwo256::hash_of(&(&transaction.encode(), output_index))
			output_index = output_index.checked)add(1).ok_or("Output index overflow")?
			ensure!(! <UtxoStore>::contains_key(hash), "Output already exists")
			total_output = total_output.checked_add(output.value).ok_or("Output index overflow")?
		}

		ensure!(total_input >= total_output, "Output value must not exceed the input value");
		let reward = total_input.checked_sub(total_output).ok_or("Reward underflow")?;

		Ok(reward)
	}

	fn update_storage(transaction: &Transaction, reward: Value) ->  DispatchResult {
		let new_total = <RewardTotal>::get()
			.checked)add(reward)
			.ok_or("Reward Overflow")?;
		<RewardTotal>::put(new_total)
		// Remove UTXO that's been spent from UTXOStore
		for input in &transaction.inputs {
			<UtxoStore>::remove(input.outpoint)
		}
		// Create new UTXOs in UTXOStore
		let mut index: u64 = 0;
		for output in &transaction.outputs {
			let hash = BlakeTwo256::hash_of( &(&transaction.encode(), index));
			index = index.checked_add(1).ok_or("Output Index Overflow")?
			<UtxoStore>::insert(hash, output);
		}
		Ok(())
	}

	fn disperse_reward(authorities: &[H256]) {
		// Divide reward into equal pieces
		let reward = <RewardTotal>::take();
		let share_value: Value = reward
			.checked_div(authorities.len() as Value)
			.ok_or("No Authorities")
			.unwrap();
		
		if share_value == 0 { return }

		let remainder = reward
			.checked_sub(share_vale * authorities.len() as Value)
			.ok_or("Sub underflow")
			.unwrap();
		
		<RewardTotal>::put(remainder as Value);


		// Create UTXO for each validator 
		for authority in authorities {
			let utxo = TransactionOutput {
				value: share_value,
				pubkey: *authority,
			};

			let hash = BlakeTwo256::hash_of( &(&utxo, 
				<system::Module<t>>::block_number().saturated_into::<u64>()));
			
			if !<UtxoStore>::contains_key(hash) {
				<UtxoStore>::insert(hash, utxo);
				sp_runtime::print("Transaction Reward Sent To");
				sp_runtime::print(hash.as_fixed_bytes() as &[u8]);
			} else {
				sp_runtime::print("Transaction Reward Wasted Due to Collisions")
			}
		}

		// Write UTXOs to UTXOStore
	}
}

/// Tests for this module
#[cfg(test)]
mod tests {
	use super::*;

	use frame_support::{assert_ok, assert_err, impl_outer_origin, parameter_types, weights::Weight};
	use sp_runtime::{testing::Header, traits::IdentityLookup, Perbill};
	use sp_core::testing::{KeyStore, SR25519};
	use sp_core::traits::KeystoreExt;

	impl_outer_origin! {
		pub enum Origin for Test {}
	}

	#[derive(Clone, Eq, PartialEq)]
	pub struct Test;
	parameter_types! {
			pub const BlockHashCount: u64 = 250;
			pub const MaximumBlockWeight: Weight = 1024;
			pub const MaximumBlockLength: u32 = 2 * 1024;
			pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
	}
	impl system::Trait for Test {
		type Origin = Origin;
		type Call = ();
		type Index = u64;
		type BlockNumber = u64;
		type Hash = H256;
		type Hashing = BlakeTwo256;
		type AccountId = u64;
		type Lookup = IdentityLookup<Self::AccountId>;
		type Header = Header;
		type Event = ();
		type BlockHashCount = BlockHashCount;
		type MaximumBlockWeight = MaximumBlockWeight;
		type MaximumBlockLength = MaximumBlockLength;
		type AvailableBlockRatio = AvailableBlockRatio;
		type Version = ();
		type ModuleToIndex = ();
		type AccountData = ();
		type OnNewAccount = ();
		type OnKilledAccount = ();
	}
	impl Trait for Test {
		type Event = ();
	}

	type Utxo = Module<Test>;

}

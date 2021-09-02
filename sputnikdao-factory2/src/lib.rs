use std::str::FromStr;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedSet;
use near_sdk::json_types::{Base64VecU8, U128};
use near_sdk::{assert_self, env, ext_contract, near_bindgen, AccountId, Gas, Promise, PublicKey};

const CODE: &[u8] = include_bytes!("../../sputnikdao2/res/sputnikdao2.wasm");

/// Gas spent on the call & account creation.
const CREATE_CALL_GAS: Gas = Gas(75_000_000_000_000);

/// Gas allocated on the callback.
const ON_CREATE_CALL_GAS: Gas = Gas(10_000_000_000_000);

#[ext_contract(ext_self)]
pub trait ExtSelf {
    fn on_create(
        &mut self,
        account_id: AccountId,
        attached_deposit: U128,
        predecessor_account_id: AccountId,
    ) -> bool;
}

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize)]
pub struct SputnikDAOFactory {
    daos: UnorderedSet<AccountId>,
}

impl Default for SputnikDAOFactory {
    fn default() -> Self {
        env::panic_str("SputnikDAOFactory should be initialized before usage")
    }
}

#[near_bindgen]
impl SputnikDAOFactory {
    #[init]
    pub fn new() -> Self {
        assert!(!env::state_exists(), "The contract is already initialized");
        Self {
            daos: UnorderedSet::new(b"d".to_vec()),
        }
    }

    pub fn get_dao_list(&self) -> Vec<AccountId> {
        self.daos.to_vec()
    }

    #[payable]
    pub fn create(
        &mut self,
        name: AccountId,
        public_key: Option<PublicKey>,
        args: Base64VecU8,
    ) -> Promise {
        let account_id =
            AccountId::from_str(&format!("{}.{}", name, env::current_account_id())).unwrap();
        let mut promise = Promise::new(account_id.clone())
            .create_account()
            .deploy_contract(CODE.to_vec())
            .transfer(env::attached_deposit());
        if let Some(key) = public_key {
            promise = promise.add_full_access_key(key.into())
        }
        promise
            .function_call(
                "new".to_string(),
                args.into(),
                0,
                env::prepaid_gas() - CREATE_CALL_GAS - ON_CREATE_CALL_GAS,
            )
            .then(ext_self::on_create(
                account_id,
                U128(env::attached_deposit()),
                env::predecessor_account_id(),
                env::current_account_id(),
                0,
                ON_CREATE_CALL_GAS,
            ))
    }

    pub fn on_create(
        &mut self,
        account_id: AccountId,
        attached_deposit: U128,
        predecessor_account_id: AccountId,
    ) -> bool {
        assert_self();
        if near_sdk::is_promise_success() {
            self.daos.insert(&account_id);
            true
        } else {
            Promise::new(predecessor_account_id).transfer(attached_deposit.0);
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use near_sdk::test_utils::{accounts, testing_env_with_promise_results, VMContextBuilder};
    use near_sdk::{testing_env, MockedBlockchain, PromiseResult};

    use super::*;

    #[test]
    fn test_basics() {
        let mut context = VMContextBuilder::new();
        let mocked_blockchain = MockedBlockchain::new(
            context.build(),
            Default::default(),
            Default::default(),
            vec![],
            Default::default(),
            Default::default(),
            None,
        );
        near_sdk::env::set_blockchain_interface(mocked_blockchain);
        testing_env!(context.current_account_id(accounts(0)).build());
        let mut factory = SputnikDAOFactory::new();
        testing_env!(context.attached_deposit(10).build());
        factory.create(
            AccountId::from_str("test").unwrap(),
            Some(
                PublicKey::from_str("ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp")
                    .unwrap(),
            ),
            "{}".as_bytes().to_vec().into(),
        );
        testing_env_with_promise_results(
            context.predecessor_account_id(accounts(0)).build(),
            PromiseResult::Successful(vec![]),
        );
        factory.on_create(
            AccountId::from_str(&format!("test.{}", accounts(0))).unwrap(),
            U128(10),
            accounts(0),
        );
        assert_eq!(
            factory.get_dao_list(),
            vec![AccountId::from_str(&format!("test.{}", accounts(0))).unwrap()]
        );
    }
}

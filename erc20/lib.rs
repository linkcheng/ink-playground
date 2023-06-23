#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod erc20 {
    // use ink::primitives::AccountId;
    use ink::storage::Mapping;

    /// Defines the storage of your contract.
    /// Add new fields to the below struct in order
    /// to add new static storage fields to your contract.
    #[ink(storage)]
    #[derive(Default)]
    pub struct Erc20 {
        total_supply: Balance,
        balances: Mapping<AccountId, Balance>,
        allowances: Mapping<(AccountId, AccountId), Balance>,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        BalanceTooLow,
        AllowanceTooLow,
    }

    #[ink(event)]
    pub struct Transfer {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        value: Balance,
    }

    #[ink(event)]
    pub struct Approval {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        value: Balance,
    }

    type Result<T> = core::result::Result<T, Error>;

    impl Erc20 {
        /// Constructor that initializes
        #[ink(constructor)]
        pub fn new(total_supply: Balance) -> Self {
            let mut balances = Mapping::new();
            let receiver = Self::env().caller();
            balances.insert(receiver, &total_supply);

            Self::env().emit_event(
                Transfer {
                    from: None,
                    to: Some(receiver),
                    value: total_supply,
                }
            );

            Self { 
                total_supply,
                balances,
                ..Default::default()
            }
        }

        #[ink(message)]
        pub fn total_supply(&self) -> Balance {
            self.total_supply
        }

        #[ink(message)]
        pub fn balance_of(&self, who: AccountId) -> Balance {
            self.balances.get(&who).unwrap_or_default()
        }

        pub fn transfer_from_to(&mut self, from: &AccountId, to: &AccountId, value: Balance) -> Result<()> {
            let balance_from = self.balance_of(*from);
            let balance_to: u128 = self.balance_of(*to);

            if value > balance_from {
                return Err(Error::BalanceTooLow)
            }

            self.balances.insert(&from, &(balance_from - value));
            self.balances.insert(&to, &(balance_to + value));

            self.env().emit_event(
                Transfer {
                    from: Some(*from),
                    to: Some(*to),
                    value,
                }
            );

            Ok(())
        }

        #[ink(message)]
        pub fn transfer(&mut self, to: AccountId, value: Balance) -> Result<()> {
            let from = self.env().caller();
            self.transfer_from_to(&from, &to, value)
        }

        #[ink(message)]
        pub fn transfer_from(&mut self, from: AccountId, to: AccountId, value: Balance) -> Result<()> {
            let sender = self.env().caller();
            let allowance = self.allowances.get(&(from, sender)).unwrap_or_default();
            if allowance < value {
                return Err(Error::AllowanceTooLow);
            }
            
            self.allowances.insert(&(from, sender), &(allowance - value));

            self.transfer_from_to(&from, &to, value)
        }

        #[ink(message)]
        pub fn approve(&mut self, spender: AccountId, value: Balance) -> Result<()> {
            let owner = self.env().caller();
            self.allowances.insert(&(owner, spender), &value);

            self.env().emit_event(
                Approval {
                    from: Some(owner),
                    to: Some(spender),
                    value,
                }
            );
            Ok(())
        }
    }

    /// Unit tests in Rust are normally defined within such a `#[cfg(test)]`
    /// module and test functions are marked with a `#[test]` attribute.
    /// The below code is technically just normal Rust code.
    #[cfg(test)]
    mod tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        type Event = <Erc20 as ::ink::reflect::ContractEventBase>::Type;

        #[ink::test]
        fn constructor_works() {
            let erc20 = Erc20::new(10000);
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            
            assert_eq!(erc20.total_supply(), 10000);
            assert_eq!(erc20.balance_of(accounts.alice), 10000);

            let emit_events = ink::env::test::recorded_events().collect::<Vec<_>>();
            let event = &emit_events[0];
            let decoded = <Event as scale::Decode>::decode(&mut &event.data[..]).expect("decoded error");
            match decoded {
                Event::Transfer(Transfer{ from, to, value }) => {
                    assert_eq!(from, None);
                    assert_eq!(to, Some(accounts.alice));
                    assert_eq!(value, 10000);
                },
                _ => panic!("expect Transfer event"),
            }
        }

        #[ink::test]
        fn transfer_should_work() {
            let mut erc20 = Erc20::new(10000);
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            assert_eq!(erc20.balance_of(accounts.bob), 0);
            assert_eq!(erc20.balance_of(accounts.alice), 10000);

            assert_eq!(erc20.transfer(accounts.bob, 100), Ok(()));
            assert_eq!(erc20.balance_of(accounts.bob), 100);
            assert_eq!(erc20.balance_of(accounts.alice), 10000-100);
        }

        #[ink::test]
        fn transfer_should_fail() {
            let mut erc20 = Erc20::new(10000);
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);

            let res = erc20.transfer(accounts.charlie, 100);
            assert!(res.is_err());
            assert_eq!(res, Err(Error::BalanceTooLow));
        }

    }


    /// This is how you'd write end-to-end (E2E) or integration tests for ink! contracts.
    ///
    /// When running these you need to make sure that you:
    /// - Compile the tests with the `e2e-tests` feature flag enabled (`--features e2e-tests`)
    /// - Are running a Substrate node which contains `pallet-contracts` in the background
    #[cfg(all(test, feature = "e2e-tests"))]
    mod e2e_tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        /// A helper function used for calling contract messages.
        use ink_e2e::build_message;

        /// The End-to-End test `Result` type.
        type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

        /// We test that we can upload and instantiate the contract using its default constructor.
        #[ink_e2e::test]
        async fn e2e_transfer(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
            let total_suply = 123;
            let constructor = Erc20Ref::new(total_suply);
            let contract_account_id = client
                .instantiate("erc20", &ink_e2e::alice(), constructor, 0, None)
                .await
                .expect("instantiate failed")
                .account_id;
            let alice_acc = ink_e2e::account_id(ink_e2e::AccountKeyring::Alice);
            let bob_acc = ink_e2e::account_id(ink_e2e::AccountKeyring::Bob);
            let transfer_msg = build_message::<Erc20Ref>(
                contract_account_id.clone()
            ).call(|erc20| erc20.transfer(bob_acc, 2));

            let ret = client.call(
                &ink_e2e::alice(), transfer_msg, 0, None
            ).await;

            assert!(ret.is_ok());

            let balance_of_msg = build_message::<Erc20Ref>(contract_account_id.clone())
                .call(|erc20| erc20.balance_of(alice_acc));
            let balance_of_alice = client.call_dry_run(
                &ink_e2e::alice(), &balance_of_msg, 0, None
            ).await;

            assert!(balance_of_alice.return_value() == 123-2);

            Ok(())
        }
    }
}

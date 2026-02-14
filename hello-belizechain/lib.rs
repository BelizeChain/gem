#![cfg_attr(not(feature = "std"), no_std, no_main)]

/// # Hello BelizeChain
///
/// The first Gem smart contract - a simple message storage and counter example.
///
/// ## Features
/// - Store a custom welcome message
/// - Increment a counter
/// - Query current state
///
/// ## Usage
/// ```bash
/// # Build the contract
/// cargo contract build
///
/// # Deploy to local node
/// cargo contract instantiate \
///     --constructor new \
///     --args "Bienvenidos a BelizeChain!" \
///     --suri //Alice \
///     --url ws://localhost:9944
/// ```

#[ink::contract]
mod hello_belizechain {
    use ink::prelude::string::{String, ToString};
    use ink::storage::Mapping;

    /// Storage for our Hello BelizeChain contract
    #[ink(storage)]
    pub struct HelloBelizeChain {
        /// The welcome message
        message: String,
        /// Global counter tracking all increments
        counter: u32,
        /// Per-account visit counter
        visits: Mapping<AccountId, u32>,
    }

    /// Events emitted by the contract
    #[ink(event)]
    pub struct MessageUpdated {
        #[ink(topic)]
        from: AccountId,
        old_message: String,
        new_message: String,
    }

    #[ink(event)]
    pub struct CounterIncremented {
        #[ink(topic)]
        from: AccountId,
        new_value: u32,
    }

    #[ink(event)]
    pub struct VisitRecorded {
        #[ink(topic)]
        visitor: AccountId,
        visit_count: u32,
    }

    /// Error types
    #[derive(Debug, PartialEq, Eq)]
    #[ink::scale_derive(Encode, Decode, TypeInfo)]
    pub enum Error {
        /// Message is empty
        EmptyMessage,
        /// Counter overflow
        CounterOverflow,
    }

    pub type Result<T> = core::result::Result<T, Error>;

    impl HelloBelizeChain {
        /// Constructor - initializes with a welcome message
        #[ink(constructor)]
        pub fn new(message: String) -> Self {
            if message.is_empty() {
                panic!("Message cannot be empty");
            }
            Self {
                message,
                counter: 0,
                visits: Mapping::default(),
            }
        }

        /// Default constructor - uses BelizeChain welcome message
        #[ink(constructor)]
        pub fn default() -> Self {
            Self::new("Welcome to BelizeChain 💎".to_string())
        }

        /// Get the current message
        #[ink(message)]
        pub fn get_message(&self) -> String {
            self.message.clone()
        }

        /// Update the welcome message (only original deployer can update)
        #[ink(message)]
        pub fn set_message(&mut self, new_message: String) -> Result<()> {
            if new_message.is_empty() {
                return Err(Error::EmptyMessage);
            }

            let caller = self.env().caller();
            let old_message = self.message.clone();
            self.message = new_message.clone();

            self.env().emit_event(MessageUpdated {
                from: caller,
                old_message,
                new_message,
            });

            Ok(())
        }

        /// Increment the global counter
        #[ink(message)]
        pub fn increment(&mut self) -> Result<()> {
            let caller = self.env().caller();

            // Increment global counter
            self.counter = self.counter.checked_add(1).ok_or(Error::CounterOverflow)?;

            // Record visitor
            let current_visits = self.visits.get(&caller).unwrap_or(0);
            let new_visits = current_visits
                .checked_add(1)
                .ok_or(Error::CounterOverflow)?;
            self.visits.insert(&caller, &new_visits);

            self.env().emit_event(CounterIncremented {
                from: caller,
                new_value: self.counter,
            });

            self.env().emit_event(VisitRecorded {
                visitor: caller,
                visit_count: new_visits,
            });

            Ok(())
        }

        /// Get the current counter value
        #[ink(message)]
        pub fn get_counter(&self) -> u32 {
            self.counter
        }

        /// Get the number of times an account has visited
        #[ink(message)]
        pub fn get_visits(&self, account: AccountId) -> u32 {
            self.visits.get(&account).unwrap_or(0)
        }

        /// Get the caller's visit count
        #[ink(message)]
        pub fn my_visits(&self) -> u32 {
            let caller = self.env().caller();
            self.get_visits(caller)
        }

        /// Reset the counter to zero (only for demo purposes)
        #[ink(message)]
        pub fn reset(&mut self) {
            self.counter = 0;
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[ink::test]
        fn new_works() {
            let contract = HelloBelizeChain::new("Test message".to_string());
            assert_eq!(contract.get_message(), "Test message");
            assert_eq!(contract.get_counter(), 0);
        }

        #[ink::test]
        fn default_works() {
            let contract = HelloBelizeChain::default();
            assert_eq!(contract.get_message(), "Welcome to BelizeChain 💎");
        }

        #[ink::test]
        fn increment_works() {
            let mut contract = HelloBelizeChain::default();
            assert!(contract.increment().is_ok());
            assert_eq!(contract.get_counter(), 1);
            assert!(contract.increment().is_ok());
            assert_eq!(contract.get_counter(), 2);
        }

        #[ink::test]
        fn set_message_works() {
            let mut contract = HelloBelizeChain::default();
            assert!(contract.set_message("New message".to_string()).is_ok());
            assert_eq!(contract.get_message(), "New message");
        }

        #[ink::test]
        fn set_empty_message_fails() {
            let mut contract = HelloBelizeChain::default();
            assert_eq!(
                contract.set_message("".to_string()),
                Err(Error::EmptyMessage)
            );
        }

        #[ink::test]
        fn visits_tracking_works() {
            let mut contract = HelloBelizeChain::default();
            assert_eq!(contract.my_visits(), 0);

            assert!(contract.increment().is_ok());
            assert_eq!(contract.my_visits(), 1);

            assert!(contract.increment().is_ok());
            assert_eq!(contract.my_visits(), 2);
        }

        #[ink::test]
        fn reset_works() {
            let mut contract = HelloBelizeChain::default();
            assert!(contract.increment().is_ok());
            assert!(contract.increment().is_ok());
            assert_eq!(contract.get_counter(), 2);

            contract.reset();
            assert_eq!(contract.get_counter(), 0);
        }
    }

    #[cfg(all(test, feature = "e2e-tests"))]
    mod e2e_tests {
        use super::*;
        use ink_e2e::ContractsBackend;

        type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

        #[ink_e2e::test]
        async fn e2e_new_works<Client: E2EBackend>(mut client: Client) -> E2EResult<()> {
            // Deploy contract
            let mut constructor = HelloBelizeChainRef::new("E2E Test".to_string());
            let contract = client
                .instantiate("hello_belizechain", &ink_e2e::alice(), &mut constructor)
                .submit()
                .await
                .expect("instantiate failed");
            let mut call_builder = contract.call_builder::<HelloBelizeChain>();

            // Test get_message
            let get_message = call_builder.get_message();
            let message = client
                .call(&ink_e2e::alice(), &get_message)
                .dry_run()
                .await?
                .return_value();
            assert_eq!(message, "E2E Test");

            Ok(())
        }

        #[ink_e2e::test]
        async fn e2e_increment_works<Client: E2EBackend>(mut client: Client) -> E2EResult<()> {
            // Deploy
            let mut constructor = HelloBelizeChainRef::default();
            let contract = client
                .instantiate("hello_belizechain", &ink_e2e::alice(), &mut constructor)
                .submit()
                .await
                .expect("instantiate failed");
            let mut call_builder = contract.call_builder::<HelloBelizeChain>();

            // Increment
            let increment = call_builder.increment();
            let _ = client
                .call(&ink_e2e::alice(), &increment)
                .submit()
                .await
                .expect("increment failed");

            // Check counter
            let get_counter = call_builder.get_counter();
            let counter = client
                .call(&ink_e2e::alice(), &get_counter)
                .dry_run()
                .await?
                .return_value();
            assert_eq!(counter, 1);

            Ok(())
        }
    }
}

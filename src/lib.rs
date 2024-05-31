//lib.rs
use std::fmt::format;

use borsh::{BorshDeserialize, BorshSerialize};
use ethers::{
    abi::{decode, ParamType, Token},
    prelude::{parse_log, EthEvent},
};
use l1x_sdk::{
    call_contract, caller_address, contract, contract_interaction::ContractCall, store::LookupMap,
};
use serde::{Deserialize, Serialize};

const STORAGE_CONTRACT_KEY: &[u8; 4] = b"swap";
const STORAGE_EVENTS_KEY: &[u8; 6] = b"events";
const EVENT_STATUS: &[u8; 6] = b"status";
const TRANSACTION_HASH: &[u8; 16] = b"transaction-hash";
const L1X_GATEAWY: &str = "9fcadd96d17e73423b3dd59b208aa7b468474ba4";
const SUPPORTED_TOKENS: &[u8; 16] = b"supported-tokens";

/// Enumerates two types of events, SwapInitiated and SwapExecuted.
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub enum Event {
    Initiate(SwapRequest),
    Execute(SwapExecuted),
}

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct SwapRequest {
    source_amount: l1x_sdk::types::U256,
    destination_amount: l1x_sdk::types::U256,
    sender_address: l1x_sdk::types::Address,
    receiver_address: l1x_sdk::types::Address,
    source_asset_address: l1x_sdk::types::Address,
    destination_asset_address: l1x_sdk::types::Address,
    source_asset_symbol: String,
    destination_asset_symbol: String,
    source_network: String,
    destination_network: String,
    conversion_rate_id: String,
    internal_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SwapRequestSolidity {
    source_amount: ethers::types::U256,
    destination_amount: ethers::types::U256,
    sender_address: ethers::types::Address,
    receiver_address: ethers::types::Address,
    source_asset_address: ethers::types::Address,
    destination_asset_address: ethers::types::Address,
    source_asset_symbol: String,
    destination_asset_symbol: String,
    source_network: String,
    destination_network: String,
    conversion_rate_id: String,
    internal_id: String,
}

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct SwapExecuted {
    destination_amount: l1x_sdk::types::U256,
    receiver_address: l1x_sdk::types::Address,
    destination_asset_address: l1x_sdk::types::Address,
    internal_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SwapExecutedtSolidity {
    destination_amount: ethers::types::U256,
    receiver_address: ethers::types::Address,
    destination_asset_address: ethers::types::Address,
    internal_id: String,
}

#[derive(Clone, Debug, EthEvent, Serialize, Deserialize)]
#[ethevent(name = "XTalkMessageBroadcasted")]
pub struct XTalkMessageInitiated {
    message: ethers::types::Bytes,
    destination_network: String,
    destination_smart_contract_address: ethers::types::Address,
}

impl From<SwapRequestSolidity> for SwapRequest {
    fn from(event: SwapRequestSolidity) -> Self {
        Self {
            source_amount: L1XStandardCrossChainSwap::from_ethers_u256_to_l1x_u256(
                event.source_amount,
            ),
            destination_amount: L1XStandardCrossChainSwap::from_ethers_u256_to_l1x_u256(
                event.destination_amount,
            ),
            sender_address: l1x_sdk::types::Address::from(event.sender_address.0),
            receiver_address: l1x_sdk::types::Address::from(event.receiver_address.0),
            source_asset_address: l1x_sdk::types::Address::from(event.source_asset_address.0),
            destination_asset_address: l1x_sdk::types::Address::from(
                event.destination_asset_address.0,
            ),
            source_asset_symbol: event.source_asset_symbol,
            destination_asset_symbol: event.destination_asset_symbol,
            source_network: event.source_network,
            destination_network: event.destination_network,
            conversion_rate_id: event.conversion_rate_id,
            internal_id: event.internal_id,
        }
    }
}

impl From<SwapExecutedtSolidity> for SwapExecuted {
    fn from(event: SwapExecutedtSolidity) -> Self {
        Self {
            destination_amount: L1XStandardCrossChainSwap::from_ethers_u256_to_l1x_u256(
                event.destination_amount,
            ),
            receiver_address: l1x_sdk::types::Address::from(event.receiver_address.0),
            destination_asset_address: l1x_sdk::types::Address::from(
                event.destination_asset_address.0,
            ),
            internal_id: event.internal_id,
        }
    }
}

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct Payload {
    data: Vec<u8>,
    destination_network: String,
    destination_contract_address: String,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct L1XStandardCrossChainSwap {
    events: LookupMap<String, Event>,
    supported_token: LookupMap<String, bool>,
    conversion_rate_address: String,
    event_status: LookupMap<String, String>,
    transaction: LookupMap<String,String>,
    total_events: u64,
}

impl Default for L1XStandardCrossChainSwap {
    fn default() -> Self {
        Self {
            events: LookupMap::new(STORAGE_EVENTS_KEY.to_vec()),
            supported_token: LookupMap::new(SUPPORTED_TOKENS.to_vec()),
            conversion_rate_address: "".to_string(),
            event_status: LookupMap::new(EVENT_STATUS.to_vec()),
            transaction: LookupMap::new(TRANSACTION_HASH.to_vec()),
            total_events: u64::default(),
        }
    }
}

#[contract]
impl L1XStandardCrossChainSwap {
    /// Generate contract based on bytes in storage
    fn load() -> Self {
        match l1x_sdk::storage_read(STORAGE_CONTRACT_KEY) {
            Some(bytes) => match Self::try_from_slice(&bytes) {
                Ok(contract) => contract,
                Err(_) => {
                    panic!("Unable to parse contract bytes")
                }
            },
            None => {
                panic!("The contract isn't initialized")
            }
        }
    }

    /// Save contract to storage
    fn save(&mut self) {
        match borsh::BorshSerialize::try_to_vec(self) {
            Ok(encoded_contract) => {
                l1x_sdk::storage_write(STORAGE_CONTRACT_KEY, &encoded_contract);
                log::info!("Saved event data successfully");
            }
            Err(_) => panic!("Unable to save contract"),
        };
    }

    /// Instantiate and save contract to storage with default values
    pub fn new() {
        let mut contract = Self::default();
        contract.supported_token.insert("USDT".to_string(), true);
        contract.supported_token.insert("USDC".to_string(), true);
        contract.supported_token.insert("BNB".to_string(), true);
        contract.supported_token.insert("AVAX".to_string(), true);
        contract.supported_token.insert("ETH".to_string(), true);
        contract.supported_token.insert("MATIC".to_string(), true);
        contract.supported_token.insert("L1X".to_string(), true);
        contract.save();
    }

    /// Save event to contract storage
    ///
    /// - `global_tx_id`: Global transaction identifier
    /// - `source_id`: Source Identifier
    /// - `event_data`: Data to store in contract's storage
    pub fn save_event_data(event_data: Vec<u8>, global_tx_id: String) {
        l1x_sdk::msg(&format!(
            "********************global tx id {} **************",
            global_tx_id
        ));
        assert_eq!(
            caller_address(),
            l1x_sdk::types::Address::try_from(L1X_GATEAWY).unwrap(),
            "Only the owner can call this function"
        );

        let mut contract = Self::load();

        log::info!("Received event data!!!");
        assert!(!global_tx_id.is_empty(), "global_tx_id cannot be empty");
        assert!(!event_data.is_empty(), "event_data cannot be empty");
        assert!(
            !contract.events.contains_key(&global_tx_id),
            "event is saved already"
        );

        let event_data = match base64::decode(event_data) {
            Ok(data) => data,
            Err(_) => panic!("Can't decode base64 event_data"),
        };

        let log: ethers::types::Log =
            serde_json::from_slice(&event_data).expect("Can't deserialize Log object");

        l1x_sdk::msg(&format!("{:#?}", log));
        let event_id = log.topics[0].to_string();
        assert!(
            !contract
                .events
                .contains_key(&Self::to_key(global_tx_id.clone(), event_id.clone())),
            "already executed"
        );
        if let Ok(swap_initiated_event) = parse_log::<XTalkMessageInitiated>(log.clone()) {
            l1x_sdk::msg(&format!("valid log"));
            if let Ok(event_tokens) = decode(
                &[
                    ParamType::Uint(256),
                    ParamType::Uint(256),
                    ParamType::Address,
                    ParamType::Address,
                    ParamType::Address,
                    ParamType::Address,
                    ParamType::String,
                    ParamType::String,
                    ParamType::String,
                    ParamType::String,
                    ParamType::String,
                    ParamType::String,
                ],
                &swap_initiated_event.message,
            ) {
                let event = SwapRequestSolidity {
                    source_amount: event_tokens[0].clone().into_uint().unwrap(),
                    destination_amount: event_tokens[1].clone().into_uint().unwrap(),
                    sender_address: event_tokens[2].clone().into_address().unwrap(),
                    receiver_address: event_tokens[3].clone().into_address().unwrap(),
                    source_asset_address: event_tokens[4].clone().into_address().unwrap(),
                    destination_asset_address: event_tokens[5].clone().into_address().unwrap(),
                    source_asset_symbol: event_tokens[6].clone().into_string().unwrap(),
                    destination_asset_symbol: event_tokens[7].clone().into_string().unwrap(),
                    source_network: event_tokens[8].clone().into_string().unwrap(),
                    destination_network: event_tokens[9].clone().into_string().unwrap(),
                    conversion_rate_id: event_tokens[10].clone().into_string().unwrap(),
                    internal_id: event_tokens[11].clone().into_string().unwrap(),
                };
                l1x_sdk::msg(&format!("event emitted: {:#?}", event));

                Self::validate_destination_amount(
                    event.source_network.to_string(),
                    swap_initiated_event.destination_network.clone(),
                    event.source_asset_symbol.clone(),
                    event.destination_asset_symbol.clone(),
                    L1XStandardCrossChainSwap::from_ethers_u256_to_l1x_u256(event.source_amount),
                    L1XStandardCrossChainSwap::from_ethers_u256_to_l1x_u256(event.destination_amount),
                    event.conversion_rate_id.clone(),
                );

                contract.save_swap_request_event(
                    global_tx_id,
                    event_id,
                    event,
                    swap_initiated_event.destination_network,
                    swap_initiated_event.destination_smart_contract_address,
                );
            } else if let Ok(event) = decode(
                &[
                    ParamType::Uint(256),
                    ParamType::Address,
                    ParamType::Address,
                    ParamType::String,
                ],
                &swap_initiated_event.message,
            ) {
                let event = SwapExecutedtSolidity {
                    destination_amount: event[0].clone().into_uint().unwrap(),
                    receiver_address: event[1].clone().into_address().unwrap(),
                    destination_asset_address: event[2].clone().into_address().unwrap(),
                    internal_id: event[3].clone().into_string().unwrap(),
                };

                if contract.event_status.get(&event.internal_id).is_some() {
                    contract.save_swap_executed_event(global_tx_id, event_id, event);
                }
            } else {
                l1x_sdk::msg(&format!("wrong data"));
            }
        } else {
            panic!("invalid event!")
        }

        contract.save()
    }

    fn save_swap_request_event(
        &mut self,
        global_tx_id: String,
        event_id: String,
        event: SwapRequestSolidity,
        destination_network: String,
        destination_contract_address: ethers::types::Address,
    ) {
        let event_data: SwapRequest = event.clone().into();
        l1x_sdk::msg(&format!("{:#?}", event_data));
        let key = L1XStandardCrossChainSwap::to_key(global_tx_id.clone(), event_id);
        self.events.insert(key, Event::Initiate(event_data.clone()));
        self.event_status
            .insert(event_data.internal_id, "Pending".to_string());
        l1x_sdk::msg(&format!("event saved!"));
        let payload = L1XStandardCrossChainSwap::get_execute_swap_payload(
            global_tx_id,
            event,
            destination_network,
            destination_contract_address,
        );
        l1x_sdk::msg(&format!("emitted event: {:?}", payload));
        l1x_sdk::emit_event_experimental(payload);
    }

    fn save_swap_executed_event(
        &mut self,
        global_tx_id: String,
        event_id: String,
        event: SwapExecutedtSolidity,
    ) {
        let event_data: SwapExecuted = event.clone().into();
        l1x_sdk::msg(&format!("{:#?}", event_data));
        let key = L1XStandardCrossChainSwap::to_key(global_tx_id.clone(), event_id);
        self.events.insert(key, Event::Execute(event_data.clone()));
        self.event_status
            .set(event_data.internal_id, Some("Success".to_string()));
    }

    pub fn get_transaction_status(internal_id: String) -> String {
        let contract = Self::load();
        match contract.event_status.get(&internal_id) {
            Some(status) => status.clone(),
            None => "Invalid internal id".to_string(),
        }
    }

    fn from_ethers_u256_to_l1x_u256(number: ethers::types::U256) -> l1x_sdk::types::U256 {
        let mut tmp = vec![0u8; 32];
        number.to_little_endian(&mut tmp);
        let destination_amount = l1x_sdk::types::U256::from_little_endian(&tmp);
        destination_amount
    }

    pub fn to_key(global_tx_id: String, event_type: String) -> String {
        global_tx_id.to_owned() + "-" + &event_type
    }

    fn get_execute_swap_payload(
        global_tx_id: String,
        event: SwapRequestSolidity,
        destination_network: String,
        destination_contract_address: ethers::types::Address,
    ) -> Payload {
        let message_encoded = ethers::abi::encode(&[
            Token::Uint(event.destination_amount),
            Token::Address(event.receiver_address),
            Token::Address(event.destination_asset_address),
            Token::String(event.internal_id),
        ]);

        let signature = "_l1xReceive(bytes32,bytes)";
        let payload_encoded = ethers::abi::encode(&[
            Token::FixedBytes(hex::decode(global_tx_id).unwrap()),
            Token::Bytes(message_encoded),
        ]);

        let hash = ethers::utils::keccak256(signature.as_bytes());
        let selector = &hash[..4];
        let payload = [&selector[..], &payload_encoded[..]].concat();
        l1x_sdk::msg(&format!("payload --> {}", hex::encode(payload.clone())));
        return Payload {
            data: payload,
            destination_network,
            destination_contract_address: format!("{:#x}", destination_contract_address),
        };
    }

    pub fn validate_destination_amount(
        source_network: String,
        destination_network: String,
        source_asset_symbol: String,
        destination_asset_symbol: String,
        source_amount: l1x_sdk::types::U256,
        destination_amount: l1x_sdk::types::U256,
        conversion_rate_id: String,
    ) {
        let mut _destination_asset_symbol = destination_asset_symbol.clone();
        let mut _source_asset_symbol = source_asset_symbol.clone();

        if destination_asset_symbol == "ARBITRUM" || destination_asset_symbol == "OPTIMISM" {
            _destination_asset_symbol = "ETH".to_string();
        } else if source_asset_symbol == "ARBITRUM" || source_asset_symbol == "OPTIMISM" {
            _source_asset_symbol = "ETH".to_string();
        }

        if Self::load()
            .supported_token
            .get(&_destination_asset_symbol.to_uppercase())
            .is_none()
        {
            panic!("invalid destination asset symbol");
        }

        let args = {
            #[derive(Serialize)]
            struct Args {
                conversion_rate_id: String,
                source_token: String,
                destination_token: String,
            }
            Args {
                conversion_rate_id,
                source_token: _source_asset_symbol,
                destination_token: {
                    if _destination_asset_symbol == "L1X" {
                        "USDT".to_string()
                    } else {
                        _destination_asset_symbol
                    }
                },
            }
        };

        let call = ContractCall {
            contract_address: l1x_sdk::types::Address::try_from(Self::get_conversion_rate_contract()).unwrap(),
            method_name: "get_conversion_rate_by_id".to_string(),
            args: serde_json::to_vec(&args).unwrap(),
            gas_limit: 1_000_000,
            read_only: false,
        };

        let response = call_contract(&call).expect("Function returned nothing");
        let rate = serde_json::from_slice::<l1x_sdk::types::U256>(&response).unwrap();

        if destination_network == "BSC" && source_network != "BSC" {
            if destination_asset_symbol == "USDT" || destination_asset_symbol == "USDC" {
                let calculated_destination_amount =
                    source_amount * rate * 10u64.pow(12) / 100000000;
                l1x_sdk::msg(&format!(
                    "calcualted destination amount --> {}",
                    calculated_destination_amount
                ));
                if calculated_destination_amount != destination_amount {
                    panic!("invalid destination amount");
                }
            }
        } else if source_network == "BSC" && destination_network != "BSC" {
            if destination_asset_symbol == "USDT" || destination_asset_symbol == "USDC" {
                let calculated_destination_amount =
                    source_amount * rate / 10u64.pow(12) / 100000000;
                l1x_sdk::msg(&format!(
                    "calcualted destination amount --> {}",
                    calculated_destination_amount
                ));
                if calculated_destination_amount != destination_amount {
                    panic!("invalid destination amount");
                }
            }
        } else if destination_network == "L1X" && source_network != "BSC" {
            if source_asset_symbol == "USDT" || source_asset_symbol == "USDC" {
                let calculated_destination_amount =
                    source_amount * (rate * 2) / 10u64.pow(12) / 100000000;
                l1x_sdk::msg(&format!(
                    "calcualted destination amount --> {}",
                    calculated_destination_amount
                ));
                if calculated_destination_amount != destination_amount {
                    panic!("invalid destination amount");
                }
            }
        } else {
            let calculated_destination_amount = source_amount * rate / 100000000;
            l1x_sdk::msg(&format!(
                "calcualted destination amount --> {}",
                calculated_destination_amount
            ));
            if calculated_destination_amount != destination_amount {
                panic!("invalid destination amount");
            }
        }
    }

    pub fn set_conversion_rate_contract(address: String) {
        let mut contract = Self::load();
        contract.conversion_rate_address = address;
        contract.save();
    }

    pub fn get_conversion_rate_contract() -> String {
        let contract = Self::load();
        contract.conversion_rate_address
    }

    pub fn set_transaction_hash(internal_id: String, transaction_hash: String) {
        let mut contract = Self::load();
        contract.transaction.insert(internal_id,transaction_hash);
        contract.save();    
    }

    pub fn get_transaction_hash(internal_id: String) -> String {
        let contract = Self::load();
        contract.transaction.get(&internal_id).unwrap().clone()
    }
}
use borsh::{BorshDeserialize, BorshSerialize};
use lee_core::{
    account::AccountId,
    program::{PdaSeed, ReceiveInput, Response},
};
use osm_registry_core::{
    validate_entry, RegionEntry, Registry, E_BAD_TIMESTAMP, E_BATCH_TOO_BIG, E_DESERIALIZE,
    E_REGISTRY_FULL, MAX_ACCOUNT_BYTES, MAX_BATCH,
};

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
enum Instruction {
    InitRegistry,
    RegisterRegion(RegionEntry),
    RegisterBatch { entries: Vec<RegionEntry> },
}

// LEZ v0.4 actor model: the registry lives in this program's actor state on
// the registry PDA account. Every message goes to that one actor.
lee_core::actor_entrypoint!(handle_message);

fn registry_account_id(program: AccountId) -> AccountId {
    let mut seed = [0; 32];
    seed[..8].copy_from_slice(b"registry");
    AccountId::for_public_pda(&program, &PdaSeed::new(seed))
}

fn handle_message(input: &ReceiveInput, instruction: Instruction) -> Response {
    #[cfg(not(test))]
    let start = risc0_zkvm::guest::env::cycle_count();
    if let Err(code) = validate_instruction(input, &instruction) {
        fail(code, "invalid instruction or registry account");
    }
    #[cfg(not(test))]
    let count = match &instruction {
        Instruction::InitRegistry => 0,
        Instruction::RegisterRegion(_) => 1,
        Instruction::RegisterBatch { entries } => entries.len(),
    };
    #[cfg(not(test))]
    let single = matches!(instruction, Instruction::RegisterRegion(_));
    let response = match apply(instruction, &input.pre_state) {
        Some(bytes) => Response::set_state(bytes),
        None => Response::keep_state(),
    };
    #[cfg(not(test))]
    if count > 0 {
        risc0_zkvm::guest::env::log(&format!(
            "CU register_{} entries={} cycles={}",
            if single { "region" } else { "batch" },
            count,
            risc0_zkvm::guest::env::cycle_count() - start
        ));
    }
    response
}

fn validate_instruction(input: &ReceiveInput, instruction: &Instruction) -> Result<(), u32> {
    let program = input.receiver.program_account_id;
    if input.receiver.account_id != registry_account_id(program) {
        return Err(E_DESERIALIZE);
    }
    match instruction {
        Instruction::InitRegistry => Ok(()),
        Instruction::RegisterRegion(entry) => validate_entry(entry),
        Instruction::RegisterBatch { entries } => {
            if entries.is_empty() {
                return Err(E_DESERIALIZE);
            }
            if entries.len() > MAX_BATCH {
                return Err(E_BATCH_TOO_BIG);
            }
            for entry in entries {
                validate_entry(entry)?;
            }
            Ok(())
        }
    }
}

/// Returns the new registry bytes, or `None` to keep the current state.
fn apply(instruction: Instruction, pre_data: &[u8]) -> Option<Vec<u8>> {
    let mut state = if pre_data.is_empty() {
        Registry::default()
    } else {
        Registry::try_from_slice(pre_data)
            .unwrap_or_else(|_| fail(E_DESERIALIZE, "invalid registry data"))
    };
    match instruction {
        Instruction::InitRegistry if !pre_data.is_empty() => return None,
        Instruction::InitRegistry => {}
        Instruction::RegisterRegion(entry) => apply_entry(&mut state, entry),
        Instruction::RegisterBatch { entries } => {
            if entries.is_empty() {
                fail(E_DESERIALIZE, "empty batch");
            }
            if entries.len() > MAX_BATCH {
                fail(E_BATCH_TOO_BIG, "batch too large");
            }
            for entry in entries {
                if let Err(code) = validate_entry(&entry) {
                    fail(code, "invalid entry");
                }
                apply_entry(&mut state, entry);
            }
        }
    }
    let bytes =
        borsh::to_vec(&state).unwrap_or_else(|_| fail(E_DESERIALIZE, "registry encode failed"));
    if bytes.len() > MAX_ACCOUNT_BYTES {
        fail(E_REGISTRY_FULL, "registry exceeds account size limit");
    }
    Some(bytes)
}

fn apply_entry(state: &mut Registry, entry: RegionEntry) {
    if entry.timestamp <= 0 {
        fail(E_BAD_TIMESTAMP, "timestamp must be positive");
    }
    if let Err(code) = validate_entry(&entry) {
        fail(code, "invalid entry");
    }
    state.entries.insert(entry.region.clone(), entry);
}

fn fail(code: u32, message: &str) -> ! {
    panic!("E_{code}: {message}");
}

#[cfg(test)]
mod tests {
    use lee_core::account::{Actor, ActorState};

    use super::*;

    fn input(program: AccountId, account: AccountId, pre: Vec<u8>) -> ReceiveInput {
        ReceiveInput {
            receiver: Actor::new(account, program),
            from: None,
            is_authorized: false,
            pre_state: ActorState::from(pre),
            message: Vec::new(),
        }
    }

    fn registry_input(program: AccountId, pre: Vec<u8>) -> ReceiveInput {
        input(program, registry_account_id(program), pre)
    }

    #[test]
    fn register_region_upserts_registry_state() {
        let program = AccountId::new([3; 32]);
        let entry = good_entry("germany");
        let instruction = Instruction::RegisterRegion(entry.clone());
        validate_instruction(&registry_input(program, Vec::new()), &instruction).unwrap();
        let next = apply(instruction, &[]).unwrap();
        let state = Registry::try_from_slice(&next).unwrap();
        assert_eq!(state.entries.get("germany"), Some(&entry));
    }

    #[test]
    fn init_registry_preserves_existing_state() {
        let existing = borsh::to_vec(&Registry::default()).unwrap();
        assert_eq!(apply(Instruction::InitRegistry, &existing), None);
    }

    #[test]
    fn register_batch_applies_all_entries_atomically() {
        let entries = vec![good_entry("germany"), good_entry("france")];
        let next = apply(
            Instruction::RegisterBatch {
                entries: entries.clone(),
            },
            &[],
        )
        .unwrap();
        let registry = Registry::try_from_slice(&next).unwrap();
        assert_eq!(registry.entries.len(), 2);
        assert_eq!(registry.entries["germany"], entries[0]);
        assert_eq!(registry.entries["france"], entries[1]);
    }

    #[test]
    fn rejects_wrong_registry_pda() {
        let program = AccountId::new([3; 32]);
        let wrong = input(program, AccountId::new([9; 32]), Vec::new());
        assert_eq!(
            validate_instruction(&wrong, &Instruction::InitRegistry),
            Err(E_DESERIALIZE)
        );
    }

    #[test]
    fn rejects_registry_pda_of_another_program() {
        let program = AccountId::new([3; 32]);
        let other = registry_account_id(AccountId::new([4; 32]));
        let wrong = input(program, other, Vec::new());
        assert_eq!(
            validate_instruction(&wrong, &Instruction::InitRegistry),
            Err(E_DESERIALIZE)
        );
    }

    #[test]
    fn apply_rejects_registry_growth_over_account_ceiling() {
        let mut state = Registry::default();
        let mut old = good_entry("old");
        old.cid = "x".repeat(102_100);
        state.entries.insert(old.region.clone(), old);
        let bytes = borsh::to_vec(&state).unwrap();
        assert!(bytes.len() <= MAX_ACCOUNT_BYTES);
        let result = std::panic::catch_unwind(|| {
            apply(Instruction::RegisterRegion(good_entry("germany")), &bytes)
        });
        let panic = result
            .err()
            .unwrap_or_else(|| panic!("registry growth must be rejected"));
        let message = panic
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| panic.downcast_ref::<&str>().copied())
            .unwrap_or_default();
        assert!(
            message.contains(&format!("E_{E_REGISTRY_FULL}")),
            "{message}"
        );
    }

    #[test]
    fn batch_rejects_more_than_max_batch() {
        let program = AccountId::new([3; 32]);
        let entries = (0..=MAX_BATCH)
            .map(|i| good_entry(&format!("r/{i}")))
            .collect();
        assert_eq!(
            validate_instruction(
                &registry_input(program, Vec::new()),
                &Instruction::RegisterBatch { entries }
            ),
            Err(E_BATCH_TOO_BIG)
        );
    }

    fn good_entry(region: &str) -> RegionEntry {
        RegionEntry {
            region: region.into(),
            parent: None,
            level: 0,
            cid: "bafktestcid".into(),
            source_url: "https://example.test/data.pbf".into(),
            checksum: "35cff894271b8f56ddb81911dcffe089".into(),
            version: "261024".into(),
            hosted: true,
            timestamp: 1_798_000_000,
            registered_by: [7; 32],
            record_version: 1,
        }
    }
}

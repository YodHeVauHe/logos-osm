use borsh::{BorshDeserialize, BorshSerialize};
use lee_core::{
    account::{AccountId, ShardData},
    program::{run_program, AccountMeta, PdaSeed, Plan, PlanInput},
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

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
enum Effect {
    InitRegistry,
    RegisterRegion(RegionEntry),
    RegisterBatch { entries: Vec<RegionEntry> },
}

fn main() {
    run_program(plan, apply)
}

fn registry_account_id(program: AccountId) -> AccountId {
    let mut seed = [0; 32];
    seed[..8].copy_from_slice(b"registry");
    AccountId::for_public_pda(&program, &PdaSeed::new(seed))
}

fn registry_meta(program: AccountId) -> AccountMeta {
    AccountMeta::new(registry_account_id(program), false, program)
}

fn plan(input: &PlanInput, instruction: Instruction) -> Plan {
    #[cfg(not(test))]
    let start = risc0_zkvm::guest::env::cycle_count();
    let code = validate_instruction(input, &instruction);
    if let Err(code) = code {
        fail(code, "invalid instruction or registry account");
    }
    let [registry] = input.accounts.as_slice() else {
        fail(E_DESERIALIZE, "expected registry account");
    };
    let effect = match instruction {
        Instruction::InitRegistry => Effect::InitRegistry,
        Instruction::RegisterRegion(entry) => Effect::RegisterRegion(entry),
        Instruction::RegisterBatch { entries } => Effect::RegisterBatch { entries },
    };
    let mut plan = Plan::new(input);
    plan.effect(registry, &effect);
    #[cfg(not(test))]
    {
        let count = match &effect {
            Effect::InitRegistry => 0,
            Effect::RegisterRegion(_) => 1,
            Effect::RegisterBatch { entries } => entries.len(),
        };
        if count > 0 {
            risc0_zkvm::guest::env::log(&format!(
                "CU register_{} entries={} cycles={}",
                if count == 1 && matches!(effect, Effect::RegisterRegion(_)) {
                    "region"
                } else {
                    "batch"
                },
                count,
                risc0_zkvm::guest::env::cycle_count() - start
            ));
        }
    }
    #[cfg(test)]
    let _ = instruction;
    #[cfg(not(test))]
    let _ = start;
    plan
}

fn validate_instruction(input: &PlanInput, instruction: &Instruction) -> Result<(), u32> {
    let [registry] = input.accounts.as_slice() else {
        return Err(E_DESERIALIZE);
    };
    if registry.account_id != registry_account_id(input.self_account_id)
        || registry.program_account_id != input.self_account_id
    {
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

fn apply(effect: Effect, pre_data: &ShardData) -> Option<Vec<u8>> {
    let mut state = if pre_data.is_empty() {
        Registry::default()
    } else {
        Registry::try_from_slice(pre_data)
            .unwrap_or_else(|_| fail(E_DESERIALIZE, "invalid registry data"))
    };
    match effect {
        Effect::InitRegistry if !pre_data.is_empty() => return None,
        Effect::InitRegistry => {}
        Effect::RegisterRegion(entry) => apply_entry(&mut state, entry),
        Effect::RegisterBatch { entries } => {
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
    use super::*;

    #[test]
    fn register_region_upserts_registry_shard() {
        let program = AccountId::new([3; 32]);
        let registry = registry_meta(program);
        let entry = good_entry("germany");
        let input = PlanInput {
            self_account_id: program,
            caller_account_id: None,
            accounts: vec![registry],
            instruction_data: borsh::to_vec(&Instruction::RegisterRegion(entry.clone())).unwrap(),
        };
        let plan = plan(&input, Instruction::RegisterRegion(entry.clone()));
        let effect = borsh::from_slice::<Effect>(&plan.output().effects[0].data).unwrap();
        let next = apply(effect, &ShardData::empty()).unwrap();
        let state = Registry::try_from_slice(&next).unwrap();
        assert_eq!(state.entries.get("germany"), Some(&entry));
    }

    #[test]
    fn init_registry_preserves_existing_state() {
        let existing = borsh::to_vec(&Registry::default()).unwrap();
        assert_eq!(
            apply(
                Effect::InitRegistry,
                &ShardData::try_from(existing.clone()).unwrap()
            ),
            None
        );
    }

    #[test]
    fn batch_rejects_more_than_max_batch() {
        let program = AccountId::new([3; 32]);
        let entries = (0..=MAX_BATCH)
            .map(|i| good_entry(&format!("r/{i}")))
            .collect();
        let input = PlanInput {
            self_account_id: program,
            caller_account_id: None,
            accounts: vec![registry_meta(program)],
            instruction_data: Vec::new(),
        };
        assert_eq!(
            validate_instruction(&input, &Instruction::RegisterBatch { entries }),
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

use anchor_lang::prelude::*;

pub fn lamports_to_sol(lamports: u64) -> f64 {
    lamports as f64 / 1_000_000_000.0
}

pub fn sol_to_lamports(sol: f64) -> u64 {
    (sol * 1_000_000_000.0) as u64
}

// Add this function to your tss.rs or create a separate transaction module
pub fn create_unsigned_transaction(
    amount: f64,
    to: &Pubkey,
    memo: Option<String>,
    from: &Pubkey,
) -> Transaction {
    use solana_sdk::{system_instruction, transaction::Transaction};

    let lamports = (amount * 1_000_000_000.0) as u64;
    let mut instructions = vec![system_instruction::transfer(from, to, lamports)];

    if let Some(memo_text) = memo {
        let memo_instruction = solana_program::instruction::Instruction::new_with_bytes(
            spl_memo::id(),
            memo_text.as_bytes(),
            vec![],
        );
        instructions.push(memo_instruction);
    }

    Transaction::new_with_payer(&instructions, Some(from))
}

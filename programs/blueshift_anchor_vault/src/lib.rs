use anchor_lang::prelude::*;
declare_id!("22222222222222222222222222222222222222222222");

#[program]
pub mod blueshift_anchor_vault {
    use super::*;
    use anchor_lang::system_program::{transfer, Transfer};

    // instruction to deposit SOL into a vault (vault is a PDA owned by this program)
    pub fn deposit(ctx: Context<VaultAction>, amount: u64) -> Result<()> {
        // Check if vault is empty
        require_eq!(
            ctx.accounts.vault.lamports(),
            0,
            VaultError::VaultAlreadyExists
        );

        // Ensure amount exceeds rent-exempt minimum
        // solana accounts need a min balance to avoid being deleted
        require_gt!(
            amount,
            Rent::get()?.minimum_balance(0),
            VaultError::InvalidAmount
        );

        // transfer SOL from user wallet to vault PDA
        // this is a cpi (cross-program-invocation) to system program
        transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(), //system program
                Transfer {
                    from: ctx.accounts.signer.to_account_info(), //user wallet
                    to: ctx.accounts.vault.to_account_info(),    // vault PDA
                },
            ),
            amount, // amount to be transferred
        )?;

        Ok(())
    }

    // instruction to withdraw all SOL from vault PDA to user
    pub fn withdraw(ctx: Context<VaultAction>) -> Result<()> {
        // make sure wallet has some SOL to withdraw
        require_neq!(ctx.accounts.vault.lamports(), 0, VaultError::InvalidAmount);

        // Build signer seeds for vault PDA
        // needed so the program can sign behalf of the vault (PDA has no private key)
        let signer_key = ctx.accounts.signer.key();

        let signer_seeds = &[b"vault", signer_key.as_ref(), &[ctx.bumps.vault]];

        //transfer all SOL from vault to user
        // use CpiContext::new_with_signer to authorize the vault PDA to send funds
        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.system_program.to_account_info(), // system program
                Transfer {
                    from: ctx.accounts.vault.to_account_info(), //vault PDA
                    to: ctx.accounts.signer.to_account_info(),  //user wallet
                },
                &[&signer_seeds[..]], // seeds + bump used to sign as vault
            ),
            ctx.accounts.vault.lamports(), // all SOL
        )?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct VaultAction<'info> {
    // user signing the tx (must be mutable since they are sending/receiving SOL)
    #[account(mut)]
    pub signer: Signer<'info>,

    //  Vault PDA for the signer
    // this is derived using seeds
    // PDA means this account is program owned and controlled
    #[account(
        mut,
        seeds = [b"vault", signer.key().as_ref()],
        bump
    )]
    pub vault: SystemAccount<'info>, //native SOL-holding account (not a custom struct)
    pub system_program: Program<'info, System>, //required to perform SOL transfers (built-in system program)
}

#[error_code]
pub enum VaultError {
    #[msg("Vault already exists")]
    VaultAlreadyExists,
    #[msg("Invalid amount")]
    InvalidAmount,
}

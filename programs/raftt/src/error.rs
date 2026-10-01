use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("A meta da oferta deve ser maior que zero.")]
    InvalidTargetAmount,
    
    #[msg("O valor do investimento deve ser maior que zero.")]
    InvalidInvestmentAmount,
    
    #[msg("Esta oferta já se encontra fechada.")]
    OfferingIsClosed,
    
    #[msg("O investimento ultrapassa o valor alvo da oferta.")]
    ExceedsTargetAmount,
    
    #[msg("Erro de overflow numérico.")]
    NumericalOverflow,
}
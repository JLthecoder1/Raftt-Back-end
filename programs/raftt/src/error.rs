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

    #[msg("Esta oferta não está disponível para reembolso.")]
    OfferingNotFailed,

    #[msg("Esta oferta já foi marcada como falha.")]
    OfferingAlreadyFailed,

    #[msg("A oferta ainda não atingiu a meta.")]
    OfferingNotCompleted,

    #[msg("Os fundos desta oferta já foram liberados.")]
    OfferingAlreadyReleased,

    #[msg("Ação não autorizada")]
    Unauthorized,

    #[msg("Estado incompatível com a ação")]
    InvalidState,

    #[msg("Valor ou orçamento inválido")]
    InvalidAmount,

    #[msg("Prazo inválido ou ainda não alcançado")]
    InvalidDeadline,

    #[msg("Cancelamento bloqueado nas últimas 48 horas")]
    CancellationLocked,

    #[msg("Documento ou análise sem hash")]
    MissingHash,

    #[msg("Parcela fora de ordem ou já paga")]
    InvalidMilestone,

    #[msg("Overflow numérico")]
    Overflow,
}
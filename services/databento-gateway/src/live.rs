pub mod session;

#[cfg(feature = "databento-compat")]
pub mod databento;

pub use session::{DatasetSessionManager, HandoffResolution, ResolvedStreamKeyLike};

use crate::protocol::{ProviderErrorCode, SymbolMapping};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandoffDecision {
    Continue,
    TerminateWithResolvedInstrumentChanged,
}

#[derive(Debug, Clone)]
pub struct HandoffDecisionInput {
    pub requested_symbol: String,
    pub current_instrument_id: i64,
    pub requested_instrument_id: i64,
    pub current_resolved: String,
    pub requested_resolved: String,
}

pub fn evaluate_handoff(request: HandoffDecisionInput) -> (HandoffDecision, ProviderErrorCode) {
    if request.current_instrument_id == request.requested_instrument_id
        && request.current_resolved == request.requested_resolved
    {
        (HandoffDecision::Continue, ProviderErrorCode::Internal)
    } else {
        (
            HandoffDecision::TerminateWithResolvedInstrumentChanged,
            ProviderErrorCode::ResolvedInstrumentChanged,
        )
    }
}

pub fn mapping_or_default(mapping: &Option<SymbolMapping>) -> Option<(i64, String)> {
    mapping
        .as_ref()
        .map(|entry| (entry.instrument_id, entry.resolved_symbol.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::ProviderErrorCode;

    #[test]
    fn reconnect_continues_when_resolution_is_identical() {
        let request = HandoffDecisionInput {
            requested_symbol: "ES.FUT".to_string(),
            current_instrument_id: 101,
            requested_instrument_id: 101,
            current_resolved: "ESZ4".to_string(),
            requested_resolved: "ESZ4".to_string(),
        };
        let (decision, _) = evaluate_handoff(request);
        assert_eq!(decision, HandoffDecision::Continue);
    }

    #[test]
    fn reconnect_terminates_on_instrument_change() {
        let request = HandoffDecisionInput {
            requested_symbol: "ES.FUT".to_string(),
            current_instrument_id: 101,
            requested_instrument_id: 102,
            current_resolved: "ESZ4".to_string(),
            requested_resolved: "ESZ5".to_string(),
        };
        let (decision, code) = evaluate_handoff(request);
        assert_eq!(
            decision,
            HandoffDecision::TerminateWithResolvedInstrumentChanged
        );
        assert_eq!(code, ProviderErrorCode::ResolvedInstrumentChanged);
    }
}

use hyxe_crypt::hyper_ratchet::Ratchet;
use hyxe_crypt::endpoint_crypto_container::PeerSessionCrypto;
use crate::external_services::fcm::fcm_packet_processor::FcmProcessorResult;
use crate::misc::{EmptyOptional, AccountError};

pub fn process<Fcm: Ratchet>(endpoint_crypto: &mut PeerSessionCrypto<Fcm>, truncate_vers: Option<u32>) -> Result<FcmProcessorResult, AccountError> {
    log::info!("FCM RECV TRUNCATE_ACK");
    let _ = endpoint_crypto.maybe_unlock(false).map_empty_err()?; // unconditional unlock
    if let Some(truncate_vers) = truncate_vers {
        log::info!("[FCM] Adjacent node successfully deregistered ratchet v{}", truncate_vers);
    }

    Ok(FcmProcessorResult::RequiresSave)
}
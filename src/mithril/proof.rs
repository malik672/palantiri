use bls12_381::{hash_to_curve::ExpandMsgXmd, multi_miller_loop, G1Affine, G1Projective, G2Affine, G2Prepared, G2Projective, Gt};
use mordor::SlotSynchronizer;

use crate::{shire::Forks, types::BlockHeader};

#[derive(Debug, Clone)]
pub struct BLSVerifier {
    g1_projective: G1Projective,
    g2_prepared: G2Prepared,
}


#[derive(Debug, Clone)]
pub struct HeaderVerifier {
    forks: Forks,
    bls: BLSVerifier,
}


impl HeaderVerifier {
    pub fn new() -> Self {
        Self {
            forks: Forks::new(),
            bls: BLSVerifier::default(),
        }
    }

    pub fn verify_header(&self, header: BlockHeader) -> Result<()> {
        // Get epoch from header slot
        let slot_sync = SlotSynchronizer::default();
        let epoch = slot_sync.slot_to_epoch(slot_sync.current_slot()?);

        // Verify fork version matches epoch
        self.verify_fork_version(epoch, header)?;

        // Verify execution payload based on fork
        self.verify_execution_payload(header)?;

        Ok(())
    }

    fn verify_fork_version(&self, epoch: u64, header: BlockHeader) -> Result<()> {
        let has_execution = header.execution.is_some() && header.execution_branch.is_some();

        // Pre-merge blocks shouldn't have execution payload
        if !self.forks.is_bellatrix() && has_execution {
            return Err(eyre!("Pre-merge block cannot have execution payload"));
        }

        // Match payload type with fork version
        if let Some(payload) = &header.execution {
            match payload {
                ExecutionPayloadHeader::Deneb(_) => {
                    if !self.forks.is_deneb() {
                        return Err(eyre!("Deneb payload before Deneb fork"));
                    }
                }
                ExecutionPayloadHeader::Capella(_) => {
                    if !self.forks.is_capella() || self.forks.is_deneb() {
                        return Err(eyre!("Invalid Capella payload timing"));
                    }
                }
                ExecutionPayloadHeader::Bellatrix(_) => {
                    if !self.forks.is_bellatrix() || self.forks.is_capella() {
                        return Err(eyre!("Invalid Bellatrix payload timing"));
                    }
                }
            }
        }

        Ok(())
    }

    fn verify_execution_payload(&self, header: &LightClientHeader) -> Result<()> {
        if let Some(execution) = &header.execution {
            let branch = header.execution_branch.as_ref()
                .ok_or_else(|| eyre!("Missing execution branch"))?;

            // Verify merkle proof
            self.verify_merkle_proof(
                header.beacon.body_root,
                execution.hash(), // Implement hash() for ExecutionPayloadHeader
                branch
            )?;
        }

        Ok(())
    }
}

impl BLSVerifier {
    pub fn verify_signature(
        &self,
        pubkeys: &[G1Affine],
        message: &[u8],
        signature: &G2Affine,
    ) -> Result<bool> {
        // Hash message to curve
        let msg_hash = G2Projective::hash::<ExpandMsgXmd<sha2::Sha256>>(
            message,
            b"BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_POP_",
        );

        // Prepare pairing inputs
        let signature_prepared = G2Prepared::from(*signature);
        
        // Perform pairing check
        let pairing = multi_miller_loop(&[
            (&(-self.g1_projective), &signature_prepared),
            (&pubkeys[0], &G2Prepared::from(msg_hash.to_affine())),
        ]).final_exponentiation();

        Ok(pairing == Gt::identity())
    }
}

impl Default for BLSVerifier {
    fn default() -> Self {
        Self {
            g1_projective: G1Projective::generator(),
            g2_prepared: G2Prepared::from(G2Affine::generator()),
        }
    }
}

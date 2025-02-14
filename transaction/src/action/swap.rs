use ark_ff::Zero;
use decaf377::Fr;
use penumbra_crypto::asset::Amount;
use penumbra_crypto::dex::TradingPair;
use penumbra_crypto::proofs::transparent::SwapProof;
use penumbra_crypto::{balance, dex::swap::SwapCiphertext, Balance};
use penumbra_crypto::{Note, NotePayload, Value};
use penumbra_proto::{core::dex::v1alpha1 as pb, Protobuf};

use crate::view::action_view::SwapView;
use crate::{ActionView, IsAction, TransactionPerspective};

#[derive(Clone, Debug)]
pub struct Swap {
    // A proof that this is a valid state change.
    pub proof: SwapProof,
    // Amounts will be plaintext until flow encryption is available.
    // // The encrypted amount of asset 1 to be swapped.
    // pub enc_amount_1: MockFlowCiphertext,
    // // The encrypted amount of asset 2 to be swapped.
    // pub enc_amount_2: MockFlowCiphertext,
    pub body: Body,
}

impl IsAction for Swap {
    /// Compute a commitment to the value contributed to a transaction by this swap.
    /// Will subtract (v1,t1), (v2,t2), and (f,fee_token)
    fn balance_commitment(&self) -> balance::Commitment {
        let input_1 = Value {
            amount: self.body.delta_1_i,
            asset_id: self.body.trading_pair.asset_1(),
        };
        let input_1 = -Balance::from(input_1);
        let commitment_input_1 = input_1.commit(Fr::zero());
        let input_2 = Value {
            amount: self.body.delta_2_i,
            asset_id: self.body.trading_pair.asset_2(),
        };
        let input_2 = -Balance::from(input_2);
        let commitment_input_2 = input_2.commit(Fr::zero());

        commitment_input_1 + commitment_input_2 + self.body.fee_commitment
    }

    fn view_from_perspective(&self, txp: &TransactionPerspective) -> ActionView {
        let note_commitment = self.body.swap_nft.note_commitment;

        // Get payload key for note commitment of swap NFT.

        let swap_view = if let Some(payload_key) = txp.payload_keys.get(&note_commitment) {
            // Decrypt swap NFT
            let swap_nft =
                Note::decrypt_with_payload_key(&self.body.swap_nft.encrypted_note, payload_key);

            // Decrypt swap ciphertext
            let swap_plaintext =
                SwapCiphertext::decrypt_with_payload_key(&self.body.swap_ciphertext, payload_key);

            if let (Ok(swap_nft), Ok(swap_plaintext)) = (swap_nft, swap_plaintext) {
                SwapView::Visible {
                    swap: self.to_owned(),
                    swap_nft,
                    swap_plaintext,
                }
            } else {
                SwapView::Opaque {
                    swap: self.to_owned(),
                }
            }
        } else {
            SwapView::Opaque {
                swap: self.to_owned(),
            }
        };

        ActionView::Swap(swap_view)
    }
}

impl Protobuf<pb::Swap> for Swap {}

impl From<Swap> for pb::Swap {
    fn from(s: Swap) -> Self {
        pb::Swap {
            proof: s.proof.into(),
            body: Some(s.body.into()),
        }
    }
}

impl TryFrom<pb::Swap> for Swap {
    type Error = anyhow::Error;
    fn try_from(s: pb::Swap) -> Result<Self, Self::Error> {
        Ok(Self {
            proof: s.proof[..]
                .try_into()
                .map_err(|_| anyhow::anyhow!("Swap proof malformed"))?,
            body: s
                .body
                .ok_or_else(|| anyhow::anyhow!("missing body"))?
                .try_into()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Body {
    pub trading_pair: TradingPair,
    // No commitments for the values, as they're plaintext
    // until flow encryption is available
    // pub asset_1_commitment: balance::Commitment,
    // pub asset_2_commitment: balance::Commitment,
    pub delta_1_i: Amount,
    pub delta_2_i: Amount,
    pub fee_commitment: balance::Commitment,
    // TODO: rename to note_payload
    pub swap_nft: NotePayload,
    pub swap_ciphertext: SwapCiphertext,
}

impl Protobuf<pb::SwapBody> for Body {}

impl From<Body> for pb::SwapBody {
    fn from(s: Body) -> Self {
        pb::SwapBody {
            trading_pair: Some(s.trading_pair.into()),
            delta_1_i: Some(s.delta_1_i.into()),
            delta_2_i: Some(s.delta_2_i.into()),
            fee_commitment: s.fee_commitment.to_bytes().to_vec(),
            swap_nft: Some(s.swap_nft.into()),
            swap_ciphertext: s.swap_ciphertext.0.to_vec(),
        }
    }
}

impl TryFrom<pb::SwapBody> for Body {
    type Error = anyhow::Error;
    fn try_from(s: pb::SwapBody) -> Result<Self, Self::Error> {
        Ok(Self {
            trading_pair: s
                .trading_pair
                .ok_or_else(|| anyhow::anyhow!("missing trading_pair"))?
                .try_into()?,

            delta_1_i: s
                .delta_1_i
                .ok_or_else(|| anyhow::anyhow!("missing delta_1"))?
                .try_into()?,
            delta_2_i: s
                .delta_2_i
                .ok_or_else(|| anyhow::anyhow!("missing delta_2"))?
                .try_into()?,

            fee_commitment: (&s.fee_commitment[..]).try_into()?,
            swap_nft: s
                .swap_nft
                .ok_or_else(|| anyhow::anyhow!("missing swap_nft"))?
                .try_into()?,
            swap_ciphertext: (&s.swap_ciphertext[..]).try_into()?,
        })
    }
}

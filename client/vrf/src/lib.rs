//! VRF client primitives for client-side verification

use polkadot_primitives::PARACHAIN_KEY_TYPE_ID;
use schnorrkel::keys::PublicKey;
use sp_core::sr25519::Public;
use primitives_vrf::{make_vrf_transcript, PreDigest, VrfApi};
use sp_application_crypto::ByteArray;
use sp_core::H256;
use sp_keystore::{Keystore, KeystorePtr};

/// Uses the runtime API to get the VRF inputs and sign them with the VRF key that
/// corresponds to the authoring NimbusId.
pub fn vrf_pre_digest<B, C>(
	client: &C,
	keystore: &KeystorePtr,
	key: Public,
	parent: H256,
) -> Option<sp_runtime::generic::DigestItem>
where
	B: sp_runtime::traits::Block<Hash = sp_core::H256>,
	C: sp_api::ProvideRuntimeApi<B>,
	C::Api: VrfApi<B>,
{
	let runtime_api = client.runtime_api();

	// first ? for runtime API, second ? for if last vrf output is not available

	let last_vrf_output = runtime_api.get_last_vrf_output(parent).ok()??;
	// first ? for runtime API, second ? for not VRF key associated with NimbusId
	let vrf_pre_digest = sign_vrf(last_vrf_output, key, &keystore)?;
	Some(primitives_vrf::CompatibleDigestItem::vrf_pre_digest(vrf_pre_digest))
}

/// Signs the VrfInput using the private key corresponding to the input `key` public key
/// to be found in the input keystore
fn sign_vrf(last_vrf_output: H256, key: Public, keystore: &KeystorePtr) -> Option<PreDigest> {
	let transcript = make_vrf_transcript(last_vrf_output);
	let try_sign = Keystore::sr25519_vrf_sign(
		&**keystore,
		PARACHAIN_KEY_TYPE_ID,
		&key,
		&transcript.clone().into_sign_data(),
	);
	if let Ok(Some(signature)) = try_sign {
		let public = PublicKey::from_bytes(&key.to_raw_vec()).ok()?;
		if signature
			.pre_output
			.0
			.attach_input_hash(&public, transcript.0.clone())
			.is_err()
		{
			// VRF signature cannot be validated using key and transcript
			return None;
		}
		Some(PreDigest {
			vrf_output: signature.pre_output,
			vrf_proof: signature.proof,
		})
	} else {
		// VRF key not found in keystore or VRF signing failed
		None
	}
}

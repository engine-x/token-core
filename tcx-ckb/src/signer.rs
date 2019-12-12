use failure::Fail;
use tcx_chain::{Keystore, Result, TransactionSigner};

use crate::hash::new_blake2b;
use crate::serializer::Serializer;
use crate::transaction::{CachedCell, CellInput, OutPoint, TxInput, TxOutput, Witness};
use crate::Error;
use std::collections::HashMap;
use tcx_chain::ChainSigner;

pub struct CkbTxSigner<'a> {
    ks: &'a mut dyn ChainSigner,
    symbol: &'a str,
    address: &'a str,
}

const SIGNATURE_PLACEHOLDER: [u8; 65] = [0u8; 65];

impl<'a> CkbTxSigner<'a> {
    pub fn sign_witnesses(
        &mut self,
        tx_hash: &[u8],
        witnesses: &Vec<Witness>,
        input_cells: &Vec<&CachedCell>,
    ) -> Result<Vec<Witness>> {
        // tx_hash must be 256 bit length
        if tx_hash.len() != 32 {
            return Err(Error::InvalidTxHash.into());
        }

        if witnesses.len() == 0 {
            return Err(Error::WitnessEmpty.into());
        }

        let grouped_scripts = self.group_script(input_cells)?;

        let mut raw_witnesses = witnesses.to_vec();

        for item in grouped_scripts.iter() {
            let mut ws = vec![];
            ws.extend(item.1.iter().map(|i| &witnesses[*i]));

            if witnesses.len() > input_cells.len() {
                ws.extend(&witnesses[input_cells.len()..]);
            }

            let path = &input_cells[item.1[0]].derive_path;

            let signed_witness = self.sign_witness_group(tx_hash, &ws, path)?;
            raw_witnesses[item.1[0]] = signed_witness;
        }

        Ok(raw_witnesses)
    }

    pub fn sign_witness_group(
        &mut self,
        tx_hash: &[u8],
        witness_group: &Vec<&Witness>,
        path: &str,
    ) -> Result<Witness> {
        if witness_group.len() == 0 {
            return Err(Error::WitnessGroupEmpty.into());
        }

        let first = &witness_group[0];

        let mut empty_witness = Witness {
            lock: SIGNATURE_PLACEHOLDER.to_vec(),
            input_type: first.input_type.clone(),
            output_type: first.output_type.clone(),
        };

        let serialized_empty_witness = empty_witness.serialize();
        let serialized_empty_length = serialized_empty_witness.len();

        let mut s = new_blake2b();
        s.update(tx_hash);
        s.update(&Serializer::serialize_u64(serialized_empty_length as u64));
        s.update(&serialized_empty_witness);

        for w in witness_group[1..].iter() {
            let bytes = w.serialize();
            s.update(&Serializer::serialize_u64(bytes.len() as u64));
            s.update(&bytes);
        }

        let mut result = [0u8; 32];
        s.finalize(&mut result);

        let opt_path = if path.len() > 0 { Some(path) } else { None };

        empty_witness.lock =
            self.ks
                .sign_recoverable_hash(&result, self.symbol, self.address, opt_path)?;

        Ok(empty_witness)
    }

    fn group_script(
        &mut self,
        input_cells: &Vec<&CachedCell>,
    ) -> Result<HashMap<Vec<u8>, Vec<usize>>> {
        let mut map: HashMap<Vec<u8>, Vec<usize>> = HashMap::new();

        for i in 0..input_cells.len() {
            let item = &input_cells[i];
            if item.lock.is_none() {
                continue;
            }

            let hash = item.lock.as_ref().unwrap().to_hash()?;
            let indices = map.get_mut(&hash);
            if indices.is_some() {
                indices.unwrap().push(i);
            } else {
                map.insert(hash, vec![i]);
            }
        }

        Ok(map)
    }
}

impl TransactionSigner<TxInput, TxOutput> for Keystore {
    fn sign_transaction(&mut self, symbol: &str, address: &str, tx: &TxInput) -> Result<TxOutput> {
        if tx.witnesses.len() == 0 {
            return Err(Error::RequiredWitness.into());
        }

        let find_cache_cell = |x: &OutPoint| -> Result<&CachedCell> {
            for y in tx.cached_cells.iter() {
                if y.out_point.is_some() {
                    let point = y.out_point.as_ref().unwrap();
                    if point.index == x.index && point.tx_hash == x.tx_hash {
                        return Ok(y);
                    }
                }
            }

            Err(Error::CellInputNotCached.into())
        };

        let mut input_cells: Vec<&CachedCell> = vec![];

        for x in tx.inputs.iter() {
            if x.previous_output.is_none() {
                return Err(Error::InvalidOutputPoint.into());
            }

            input_cells.push(find_cache_cell(x.previous_output.as_ref().unwrap())?);
        }

        let mut signer = CkbTxSigner {
            ks: self,
            symbol,
            address,
        };

        let signed_witnesses = signer.sign_witnesses(&tx.tx_hash, &tx.witnesses, &input_cells)?;

        let tx_output = TxOutput {
            tx_hash: tx.tx_hash.clone(),
            witnesses: signed_witnesses,
        };

        Ok(tx_output)
    }
}

#[cfg(test)]
mod tests {
    use crate::address::CkbAddress;
    use crate::transaction::{CachedCell, CellInput, OutPoint, Script, TxInput, Witness};
    use tcx_chain::{Keystore, TransactionSigner};
    use tcx_constants::{CoinInfo, CurveType};

    #[test]
    fn sign_transaction() {
        let tx_hash =
            hex::decode("4a4bcfef1b7448e27edf533df2f1de9f56be05eba645fb83f42d55816797ad2a")
                .unwrap();
        let empty_witness = Witness {
            lock: vec![],
            input_type: vec![],
            output_type: vec![],
        };
        let witnesses = vec![
            empty_witness.clone(),
            empty_witness.clone(),
            empty_witness.clone(),
            empty_witness.clone(),
        ];

        let cached_default = CachedCell {
            derive_path: "".to_string(),
            r#type: Some(Script {
                args: vec![],
                code_hash: vec![],
                hash_type: "".to_string(),
            }),
            block_hash: vec![],
            capacity: 0,
            lock: None,
            out_point: None,
            cellbase: false,
            output_data_len: 0,
            status: "".to_string(),
            data_hash: vec![],
        };

        let cached_cells = vec![
            CachedCell {
                out_point: Some({
                    OutPoint {
                        tx_hash: hex::decode(
                            "e3c3c5b5bd600803286c14acf09f47947735b25e5f5b5b546548c9ceca202cf9",
                        )
                        .unwrap(),
                        index: 0,
                    }
                }),
                lock: Some(Script {
                    args: hex::decode("edb5c73f2a4ad8df23467c9f3446f5851b5e33da").unwrap(),
                    code_hash: hex::decode(
                        "1892ea40d82b53c678ff88312450bbb17e164d7a3e0a90941aa58839f56f8df2",
                    )
                    .unwrap(),
                    hash_type: "type".to_string(),
                }),
                ..cached_default.clone()
            },
            CachedCell {
                out_point: Some({
                    OutPoint {
                        tx_hash: hex::decode(
                            "e3c3c5b5bd600803286c14acf09f47947735b25e5f5b5b546548c9ceca202cf9",
                        )
                        .unwrap(),
                        index: 1,
                    }
                }),
                lock: Some(Script {
                    args: hex::decode("e2fa82e70b062c8644b80ad7ecf6e015e5f352f6").unwrap(),
                    code_hash: hex::decode(
                        "1892ea40d82b53c678ff88312450bbb17e164d7a3e0a90941aa58839f56f8df2",
                    )
                    .unwrap(),
                    hash_type: "type".to_string(),
                }),
                ..cached_default.clone()
            },
            CachedCell {
                out_point: Some({
                    OutPoint {
                        tx_hash: hex::decode(
                            "e3c3c5b5bd600803286c14acf09f47947735b25e5f5b5b546548c9ceca202cf9",
                        )
                        .unwrap(),
                        index: 2,
                    }
                }),
                lock: Some(Script {
                    args: hex::decode("edb5c73f2a4ad8df23467c9f3446f5851b5e33da").unwrap(),
                    code_hash: hex::decode(
                        "1892ea40d82b53c678ff88312450bbb17e164d7a3e0a90941aa58839f56f8df2",
                    )
                    .unwrap(),
                    hash_type: "type".to_string(),
                }),
                ..cached_default.clone()
            },
        ];

        let inputs = vec![
            CellInput {
                previous_output: Some(OutPoint {
                    tx_hash: hex::decode(
                        "e3c3c5b5bd600803286c14acf09f47947735b25e5f5b5b546548c9ceca202cf9",
                    )
                    .unwrap(),
                    index: 0,
                }),
                since: "".to_string(),
            },
            CellInput {
                previous_output: Some(OutPoint {
                    tx_hash: hex::decode(
                        "e3c3c5b5bd600803286c14acf09f47947735b25e5f5b5b546548c9ceca202cf9",
                    )
                    .unwrap(),
                    index: 1,
                }),
                since: "".to_string(),
            },
            CellInput {
                previous_output: Some(OutPoint {
                    tx_hash: hex::decode(
                        "e3c3c5b5bd600803286c14acf09f47947735b25e5f5b5b546548c9ceca202cf9",
                    )
                    .unwrap(),
                    index: 2,
                }),
                since: "".to_string(),
            },
        ];

        let tx_input = TxInput {
            inputs,
            witnesses,
            tx_hash,
            cached_cells,
            ..TxInput::default()
        };

        let mut ks = Keystore::from_private_key(
            "dcec27d0d975b0378471183a03f7071dea8532aaf968be796719ecd20af6988f",
            "Password",
        );
        ks.unlock_by_password("Password");

        let coin_info = CoinInfo {
            coin: "CKB".to_string(),
            derivation_path: "".to_string(),
            curve: CurveType::SECP256k1,
            network: "TESTNET".to_string(),
            seg_wit: "".to_string(),
        };

        let account = ks.derive_coin::<CkbAddress>(&coin_info).unwrap().clone();

        let tx_output = ks
            .sign_transaction("CKB", &account.address, &tx_input)
            .unwrap();

        // same as the input length
        assert_eq!(tx_output.witnesses.len(), 4);
        assert_eq!(
            tx_output.witnesses[3].serialize(),
            empty_witness.serialize()
        );
        assert_eq!(
            tx_output.witnesses[2].serialize(),
            empty_witness.serialize()
        );

        assert_eq!(hex::encode(tx_output.witnesses[0].serialize()), "5500000010000000550000005500000041000000d59792eee1e67747d25a36304bf155665a9b552ecda2d84390ba6acfc422dc3f4b599165078ed98c4e930dec79866b50984f3458c5010faefce6574b9659329501");

        let mut ks = Keystore::from_private_key(
            "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
            "Password",
        );
        ks.unlock_by_password("Password");

        let coin_info = CoinInfo {
            coin: "CKB".to_string(),
            derivation_path: "".to_string(),
            curve: CurveType::SECP256k1,
            network: "TESTNET".to_string(),
            seg_wit: "".to_string(),
        };

        let account = ks.derive_coin::<CkbAddress>(&coin_info).unwrap().clone();

        let tx_output = ks
            .sign_transaction("CKB", &account.address, &tx_input)
            .unwrap();

        // same as the input length
        assert_eq!(tx_output.witnesses.len(), 4);
        assert_eq!(
            tx_output.witnesses[3].serialize(),
            empty_witness.serialize()
        );
        assert_eq!(
            tx_output.witnesses[2].serialize(),
            empty_witness.serialize()
        );

        assert_eq!(hex::encode(tx_output.witnesses[1].serialize()), "550000001000000055000000550000004100000091af5eeb1632565dc4a9fb1c6e08d1f1c7da96e10ee00595a2db208f1d15faca03332a1f0f7a0f8522f6e112bb8dde4ed0015d1683b998744a0d8644f0dfd0f800");
    }
}

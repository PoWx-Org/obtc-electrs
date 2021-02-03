use sha3::{Sha3_256, Digest};
use bitcoin::{BlockHash};
use itertools::Itertools;
use nalgebra::{U64, VectorN, U1, MatrixMN, DimMul, Matrix};
use rand_xoshiro::Xoshiro256PlusPlus;
use rand_xoshiro::rand_core::{SeedableRng, RngCore};
use std::convert::TryInto;
use bitcoin::hashes::Hash;
use bitcoin::consensus::Encodable;
use crate::chain::{BlockHeader};
use bitcoin::hashes::hex::ToHex;

pub fn heavy_hash(block: &BlockHeader) -> BlockHash {
    let mut sha3 = Sha3_256::new();
    sha3.update(block.prev_blockhash.as_ref());
    let mut seed =  <[u8; 32]>::from(sha3.finalize());

    let matrix = generateHeavyHashMatrix(seed);

    let mut input = Vec::<u8>::new();
    block.consensus_encode(&mut input);

    let mut hash = heavy_hash_internal(input, matrix);
    println!("Hash: {}", hash.to_hex());
    println!("Hash reversed: {}", hash.to_hex());
    BlockHash::from_slice(&hash).unwrap()
}

fn heavy_hash_internal(input: Vec<u8>, seed: MatrixMN<i32, U64, U64>) -> [u8; 32] {
    println!("Matrix: {:?}", seed);
    println!("Input bytes: {:?}", input);
    let mut sha_1 = Sha3_256::new();
    sha_1.update(input.as_slice());
    let mut hash1 = sha_1.finalize();
    println!("Hash1[32] {:?}", <[u8; 32]>::from(hash1));

    let mut x = hash1.iter()
        .fold(Vec::new(), |mut acc: Vec<i32>, b| {
            acc.push((*b as i32) >> 4);
            acc.push((*b as i32) & 0x0F);
            acc
        });
    println!("x[64] {:?}", x);

    let xMatrix: MatrixMN<i32, U64, U1> = VectorN::<i32, U64>::from_vec(x);
    // TRANSPOSE ???
    // let mut matrixMul = seed.transpose() * xMatrix;
    let matrixMul = seed * xMatrix;
    let y: &[i32] = matrixMul.as_slice();

    println!("y[64] {:?}", y);

    let truncated = y.iter()
        .map(|b| b >> 10)
        .collect_vec();
    println!("truncated[64] {:?}", truncated);

    let mut preout: Vec<u8> = Vec::new();
    for i in 0..(truncated.len() / 2) {
        let a = truncated.get(i << 1).unwrap();
        let b = truncated.get((i << 1) + 1).unwrap();
        let h = hash1.get(i).unwrap();

        let res = ((*a << 4) | *b) ^ (*h as i32);
        preout.push(res.try_into().unwrap());
    }
    // preout.reverse();

    println!("preout[32] {}", preout.to_hex());

    let mut sha_2 = Sha3_256::new();
    sha_2.update(preout.as_slice());
    <[u8; 32]>::from(sha_2.finalize())
}

fn generateHeavyHashMatrix(seed: [u8; 32]) -> MatrixMN<i32, U64, U64> {
    println!("Heavyhash matrix seed: {:?}", seed);
    let mut generator = Xoshiro256PlusPlus::from_seed(seed);

    loop {
        let mut matrix = MatrixMN::<i32, U64, U64>::zeros();

        for i in 0..64 {
            for j in (0..64).step_by(16) {
                let value = generator.next_u64();
                println!("Generated value: {}", value);
                for shift in 0..16 {
                    *matrix.index_mut((i, j + shift)) = ((value >> (4 * shift)) & 0xF) as i32;
                }
            }
        }

        if is4BitPrecision(&matrix) && isFullRank(&matrix) {
            return matrix;
        }
        println!("Matrix Missed! Try again");
    }
}

fn is4BitPrecision(matrix: &MatrixMN<i32, U64, U64>) -> bool {
    for i in 0..64 {
        for j in 0..64 {
            let value = matrix.get((i, j)).unwrap();
            if *value < 0 || *value > 0xF {
                return false;
            }
        }
    }
    true
}

fn isFullRank(matrix: &MatrixMN<i32, U64, U64>) -> bool {
    let mslice = matrix.as_slice();
    let fs = mslice.iter().map(|i| *i as f64).collect_vec();
    let fm = MatrixMN::<f64, U64, U64>::from_vec(fs);

    let rank = fm.rank(0.0001);
    rank == 64
}

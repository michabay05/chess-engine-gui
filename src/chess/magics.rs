use super::attack;
use super::consts::{PieceType, Sq};

fn xor_random_u32(random_state: &mut u32) -> u32 {
    let mut number = *random_state;
    // XOR shift algorithm
    number ^= number << 13;
    number ^= number >> 17;
    number ^= number << 5;

    // Update random number state
    *random_state = number;

    number
}

const XOR_RANDOM_SEED: u32 = 1804289383;

fn xor_random_u64(random_state: &mut u32) -> u64 {
    let rand1 = (xor_random_u32(random_state) & 0xFFFF) as u64;
    let rand2 = (xor_random_u32(random_state) & 0xFFFF) as u64;
    let rand3 = (xor_random_u32(random_state) & 0xFFFF) as u64;
    let rand4 = (xor_random_u32(random_state) & 0xFFFF) as u64;
    rand1 | (rand2 << 16) | (rand3 << 32) | (rand4 << 48)
}

fn gen_random_magic(random_state: &mut u32) -> u64 {
    // RANDOM U64 NUMBER WITH A SEED
    xor_random_u64(random_state) & xor_random_u64(random_state) & xor_random_u64(random_state)

    // RANDOM U64 NUMBER WITHOUT A SEED
    // rand::random::<u64>() & rand::random::<u64>() & rand::random::<u64>()
}

#[allow(unused_assignments)]
fn find_magic_number(sq: usize, relevant_bits: u32, piece: PieceType) -> u64 {
    /* Needed for the random number generation - XOR shift algorithm */
    let mut random_state: u32 = XOR_RANDOM_SEED;

    let mut used_attacks = [0u64; 4096];
    let mut occupancies = [0u64; 4096];
    let mut attacks = [0u64; 4096];
    let mut magic_num = 0u64;
    let possible_occ = if piece == PieceType::Bishop {
        attack::gen_bishop_occ(sq)
    } else {
        attack::gen_rook_occ(sq)
    };
    let occ_indices = 1 << relevant_bits;
    for ind in 0..occ_indices {
        occupancies[ind] = attack::set_occ(ind, relevant_bits, possible_occ);
        attacks[ind] = if piece == PieceType::Bishop {
            attack::gen_bishop_attack(sq, occupancies[ind])
        } else {
            attack::gen_rook_attack(sq, occupancies[ind])
        };
    }

    for _ in 0..100_000_000 {
        magic_num = gen_random_magic(&mut random_state);
        let product = u128::from(possible_occ) * u128::from(magic_num);
        if (product & 0xFF00000000000000).count_ones() < 6 {
            continue;
        }
        used_attacks = [0u64; 4096];
        let mut count = 0;
        let mut fail = false;
        while !fail && count < occ_indices {
            let num = u128::from(occupancies[count]) * u128::from(magic_num);
            let magic_ind = (num as u64) >> 64 - relevant_bits;
            if used_attacks[magic_ind as usize] == 0 {
                used_attacks[magic_ind as usize] = attacks[count];
            } else if used_attacks[magic_ind as usize] != attacks[count] {
                fail = true;
            }
            count += 1;
        }
        if !fail {
            return magic_num;
        }
    }
    println!(
        "Failed to find magic number on {}",
        Sq::to_string(Sq::from_num(sq))
    );
    return 0;
}

pub fn init() {
    println!("BISHOP: [");
    for sq in 0..64 {
        let relevant_bits = attack::BISHOP_RELEVANT_BITS[sq];
        println!(
            "    0x{:x},",
            find_magic_number(sq, relevant_bits, PieceType::Bishop)
        );
    }
    println!("];\n\nROOK: [");
    for sq in 0..64 {
        let relevant_bits = attack::ROOK_RELEVANT_BITS[sq];
        println!(
            "    0x{:x},",
            find_magic_number(sq, relevant_bits, PieceType::Rook)
        );
    }
    println!("]; ");
}

use crate::SQ;

pub type BB = u64;

pub trait BBUtil {
    fn set(&mut self, ind: usize);
    fn get(&self, ind: usize) -> bool;
    fn pop(&mut self, ind: usize);
    fn lsb(&self) -> usize;
    fn pop_lsb(&mut self) -> usize;
    fn print(&self);
}

impl BBUtil for BB {
    fn set(&mut self, ind: usize) {
        *self |= 1 << ind;
    }

    fn get(&self, ind: usize) -> bool {
        *self & (1 << ind) > 0
    }

    fn pop(&mut self, ind: usize) {
        *self ^= 1 << ind;
    }

    fn lsb(&self) -> usize {
        let num = *self ^ (*self - 1);
        let count = num.count_ones() as usize;
        if count == 0 {
            return 0;
        }
        count - 1
    }

    fn pop_lsb(&mut self) -> usize {
        let ind = self.lsb();
        self.pop(ind);
        ind
    }

    fn print(&self) {
        for r in 0..8 {
            print!(" {} |", 8 - r);
            for f in 0..8 {
                print!(" {}", if self.get(SQ!(r, f)) { '1' } else { '.' });
            }
            println!();
        }
        println!("     - - - - - - - -");
        println!("     a b c d e f g h");
    }
}

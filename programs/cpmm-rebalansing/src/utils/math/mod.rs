use anchor_lang::prelude::*;

pub mod deposit;
pub mod swap_dual;
#[cfg(feature = "test_single")]
pub mod swap_single;
pub mod withdraw;

pub use deposit::*;
pub use swap_dual::*;
#[cfg(feature = "test_single")]
pub use swap_single::*;
pub use withdraw::*;

// --- SafeMath Trait Definition ---

pub trait SafeMath: Sized {
    fn safe_add(self, rhs: Self) -> Result<Self>;
    fn safe_sub(self, rhs: Self) -> Result<Self>;
    fn safe_mul(self, rhs: Self) -> Result<Self>;
    fn safe_div(self, rhs: Self) -> Result<Self>;
    fn safe_div_ceil(self, rhs: Self) -> Result<Self>;
}

// Implementation for u128
impl SafeMath for u128 {
    #[inline(always)]
    fn safe_add(self, rhs: Self) -> Result<Self> {
        self.checked_add(rhs).ok_or(error!(MathError::MathOverflow))
    }

    #[inline(always)]
    fn safe_sub(self, rhs: Self) -> Result<Self> {
        self.checked_sub(rhs).ok_or(error!(MathError::MathOverflow))
    }

    #[inline(always)]
    fn safe_mul(self, rhs: Self) -> Result<Self> {
        self.checked_mul(rhs).ok_or(error!(MathError::MathOverflow))
    }

    #[inline(always)]
    fn safe_div(self, rhs: Self) -> Result<Self> {
        require!(rhs != 0, MathError::DivisionByZero);
        self.checked_div(rhs).ok_or(error!(MathError::MathOverflow))
    }

    #[inline(always)]
    fn safe_div_ceil(self, rhs: Self) -> Result<Self> {
        require!(rhs != 0, MathError::DivisionByZero);
        (self + rhs - 1)
            .checked_div(rhs)
            .ok_or(error!(MathError::MathOverflow))
    }
}

// Implementation for u64
impl SafeMath for u64 {
    #[inline(always)]
    fn safe_add(self, rhs: Self) -> Result<Self> {
        self.checked_add(rhs).ok_or(error!(MathError::MathOverflow))
    }

    #[inline(always)]
    fn safe_sub(self, rhs: Self) -> Result<Self> {
        self.checked_sub(rhs).ok_or(error!(MathError::MathOverflow))
    }

    #[inline(always)]
    fn safe_mul(self, rhs: Self) -> Result<Self> {
        self.checked_mul(rhs).ok_or(error!(MathError::MathOverflow))
    }

    #[inline(always)]
    fn safe_div(self, rhs: Self) -> Result<Self> {
        require!(rhs != 0, MathError::DivisionByZero);
        self.checked_div(rhs).ok_or(error!(MathError::MathOverflow))
    }

    #[inline(always)]
    fn safe_div_ceil(self, rhs: Self) -> Result<Self> {
        require!(rhs != 0, MathError::DivisionByZero);
        (self + rhs - 1)
            .checked_div(rhs)
            .ok_or(error!(MathError::MathOverflow))
    }
}

#[error_code]
pub enum MathError {
    #[msg("Math overflow")]
    MathOverflow,

    #[msg("Division by zero")]
    DivisionByZero,
}

pub fn integer_sqrt(n: u128) -> u64 {
    if n <= u64::MAX as u128 {
        return (n as u64).isqrt();
    }

    n.isqrt() as u64
}
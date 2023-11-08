use alloc::vec;
use alloc::vec::Vec;
use core::ops::{Add, AddAssign, Mul, MulAssign, Shl, ShlAssign, Shr, ShrAssign, Sub, SubAssign};

use p3_field::{PrimeField64, PrimeField32};
use crate::types::{NonCanonicalPrimeField32, IntegerLike, Canonicalize};

const MATRIX_CIRC_MDS_8_SML: [i64; 8] = [4, 1, 2, 9, 10, 5, 1, 1];

const MATRIX_CIRC_MDS_12_SML: [i64; 12] = [1, 1, 2, 1, 8, 9, 10, 7, 5, 9, 4, 10];

const MATRIX_CIRC_MDS_16_SML: [i64; 16] =
    [1, 1, 51, 1, 11, 17, 2, 1, 101, 63, 15, 2, 67, 22, 13, 3];

const P: i64 = (1 << 31) - 1;

#[rustfmt::skip]
const MATRIX_CIRC_MDS_32_MERSENNE31: [i64; 32] = [
    0x1896DC78, 0x559D1E29, 0x04EBD732, 0x3FF449D7,
    0x2DB0E2CE, 0x26776B85, 0x76018E57, 0x1025FA13,
    0x06486BAB, 0x37706EBA, 0x25EB966B, 0x113C24E5,
    0x2AE20EC4, 0x5A27507C, 0x0CD38CF1, 0x761C10E5,
    0x19E3EF1A, 0x032C730F, 0x35D8AF83, 0x651DF13B,
    0x7EC3DB1A, 0x6A146994, 0x588F9145, 0x09B79455,
    0x7FDA05EC, 0x19FE71A8, 0x6988947A, 0x624F1D31,
    0x500BB628, 0x0B1428CE, 0x3A62E1D6, 0x77692387
];

const MATRIX_CIRC_MDS_64_MERSENNE31: [i64; 64] = [
    0x570227A5, 0x3702983F, 0x4B7B3B0A, 0x74F13DE3, 0x485314B0, 0x0157E2EC, 0x1AD2E5DE, 0x721515E3,
    0x5452ADA3, 0x0C74B6C1, 0x67DA9450, 0x33A48369, 0x3BDBEE06, 0x7C678D5E, 0x160F16D3, 0x54888B8C,
    0x666C7AA6, 0x113B89E2, 0x2A403CE2, 0x18F9DF42, 0x2A685E84, 0x49EEFDE5, 0x5D044806, 0x560A41F8,
    0x69EF1BD0, 0x2CD15786, 0x62E07766, 0x22A231E2, 0x3CFCF40C, 0x4E8F63D8, 0x69657A15, 0x466B4B2D,
    0x4194B4D2, 0x1E9A85EA, 0x39709C27, 0x4B030BF3, 0x655DCE1D, 0x251F8899, 0x5B2EA879, 0x1E10E42F,
    0x31F5BE07, 0x2AFBB7F9, 0x3E11021A, 0x5D97A17B, 0x6F0620BD, 0x5DBFC31D, 0x76C4761D, 0x21938559,
    0x33777473, 0x71F0E92C, 0x0B9872A1, 0x4C2411F9, 0x545B7C96, 0x20256BAF, 0x7B8B493E, 0x33AD525C,
    0x15EAEA1C, 0x6D2D1A21, 0x06A81D14, 0x3FACEB4F, 0x130EC21C, 0x3C84C4F5, 0x50FD67C0, 0x30FDD85A,
];

//////////////////////////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////////////////////////////////

// Several functions here to encode some simple vector addition and similar.

/// Performs vector addition on slices saving the result in the first slice
#[inline]
fn add_mut<T: IntegerLike>(lhs: &mut [T], rhs: &[T]) {
    let n = rhs.len();
    for i in 0..n {
        lhs[i] += rhs[i]
    }
}

// Performs vector addition on slices returning a new array.
// This should be a const function but I can't work out how to do it.
// Need to say that Add and Default are constant. (I have a workaround for Default but not Add)
#[inline]
fn add_vec<T: IntegerLike, const N: usize>(lhs: &[T], rhs: &[T]) -> [T; N] {
    let mut output: [T; N] = [T::default(); N];
    let mut i = 0;
    loop {
        output[i] = lhs[i] + rhs[i];
        i += 1;
        if i == N {
            break;
        }
    }
    output
}

/// Performs vector subtraction on slices saving the result in the first slice
#[inline]
fn sub_mut<T: IntegerLike>(lhs: &mut [T], sub: &[T]) {
    let n = sub.len();
    for i in 0..n {
        lhs[i] -= sub[i]
    }
}

// Performs vector addition on slices returning a new array.
// This should be a const function but I can't work out how to do it.
// Need to say that Sub and Default are constant. (I have a workaround for Default but not Sub)
#[inline]
fn sub_vec<T: IntegerLike, const N: usize>(lhs: &[T], sub: &[T]) -> [T; N] {
    let mut output: [T; N] = [T::default(); N];
    let mut i = 0;
    loop {
        output[i] = lhs[i] - sub[i];
        i += 1;
        if i == N {
            break;
        }
    }
    output
}

/// Takes the dot product of two vectors whose products would overflow an i64. 
/// Computes the result as i128's and returns that.
#[inline]
fn dot_i128<T: NonCanonicalPrimeField32>(lhs: &[T], rhs: &[T]) -> i128 {
    let n = lhs.len();
    let mut sum = lhs[0] * rhs[0];
    for i in 1..n {
        sum += lhs[i] * rhs[i];
    }
    sum
}

/// Takes the dot product of two vectors which we are sure will not overflow an i64. 
#[inline]
fn dot_i64<T: NonCanonicalPrimeField32>(lhs: &[T], rhs: &[T]) -> T {
    let n = lhs.len();
    let mut sum = T::mul_small(lhs[0], rhs[0]);
    for i in 1..n {
        sum += T::mul_small(lhs[i], rhs[i]);
    }
    sum
}

/// Split an array of length N into two subarrays of length N/2.
/// Return the sum and difference of the arrays.
fn split_add_sub<T: IntegerLike, const N: usize, const HALF: usize>(
    input: [T; N],
) -> ([T; HALF], [T; HALF]) {
    // N = 2*HALF

    let (input_left, input_right) = input.split_at(HALF);

    let input_p = add_vec(input_left, input_right); // input(x) mod x^16 - 1
    let input_m = sub_vec(input_left, input_right); // input(x) mod x^16 + 1

    (input_p, input_m)
}

/// This will package all our basic convolution functions but allow for us to slightly modify implementations
/// to suit our purposes.
trait Convolution {
    /// Compute the convolution of two vectors of length 4.
    /// output(x) = lhs(x)rhs(x) mod x^4 - 1
    fn conv4<T: NonCanonicalPrimeField32>(lhs: [T; 4], rhs: [T; 4], output: &mut [T]);

    /// Compute the signed convolution of two vectors of length 4.
    /// output(x) = lhs(x)rhs(x) mod x^4 + 1
    fn signed_conv4<T: NonCanonicalPrimeField32>(lhs: &[T; 4], rhs: &[T; 4]) -> [T; 4];

    /// Compute the signed convolution of two vectors of length 4 and save in output.
    /// output(x) = lhs(x)rhs(x) mod x^4 + 1
    fn signed_conv4_mut<T: NonCanonicalPrimeField32>(lhs: &[T; 4], rhs: &[T; 4], output: &mut [T]);

    /////////////////////////////////////////////////////////////////////////////////////////////////////
    // Length 8

    /// Compute the convolution of 2 vectors of length 8.
    /// output(x) = lhs(x)rhs(x) mod x^8 - 1  <=>  output = lhs * rhs
    /// Use the FFT Trick to split into a convolution of length 4 and a signed convolution of length 4.
    #[inline]
    fn conv8<T: NonCanonicalPrimeField32>(lhs: [T; 8], rhs: [T; 8], output: &mut [T]) {
        const N: usize = 8;
        const HALF: usize = N / 2;

        // Compute lhs(x) mod x^4 - 1, lhs(x) mod x^4 + 1
        let (lhs_p, lhs_m) = split_add_sub(lhs);

        // rhs will always be constant. Not sure how to tell the compiler this though.
        // Compute rhs(x) mod x^4 - 1, rhs(x) mod x^4 + 1
        let (rhs_p, rhs_m) = split_add_sub(rhs);

        let (left, right) = output.split_at_mut(HALF);

        Self::signed_conv4_mut(&lhs_m, &rhs_m, left); // left = w_1 = lhs*rhs mod x^4 + 1
        Self::conv4(lhs_p, rhs_p, right); // right = w_0 = lhs*rhs mod x^4 - 1

        for i in 0..HALF {
            left[i] += right[i]; // w_0 + w_1
            left[i] >>= 1; // (w_0 + w_1)/2
            right[i] -= left[i]; // (w_0 - w_1)/2
        }
    }

    /// Compute the signed convolution of 2 vectors of length 8.
    /// output(x) = lhs(x)rhs(x) mod x^8 + 1
    /// Use the Karatsuba Method to split into 3 degree 3 polynomial multiplications.
    #[inline]
    fn signed_conv8<T: NonCanonicalPrimeField32>(lhs: &[T; 8], rhs: &[T; 8]) -> [T; 8] {
        const N: usize = 8;
        const HALF: usize = N / 2;

        // The algorithm is relatively simple:
        // v(x)u(x) mod x^8 + 1 = (v_e(x^2) + xv_o(x^2))(u_e(x^2) + xu_o(x^2)) mod x^8 + 1
        //          = v_e(x^2)u_e(x^2) + x^2 v_o(x^2)u_o(x^2) + x((v_e(x^2) + v_o(x^2))(u_e(x^2) + u_o(x^2)) - v_e(x^2)u_e(x^2) - v_o(x^2)u_o(x^2))

        // Now computing v_e(x^2)u_e(x^2) mod x^8 + 1 is equivalent to computing v_e(x)u_e(x) mod x^4 + 1 and similarly for the other products.

        // Clearly there should be a cleaner way to get this decomposition but everything I've tried has been slower.
        // Also seems like we are doing quite a bit of data fiddiling. Would be nice to avoid this.
        let mut lhs_even = [lhs[0], lhs[2], lhs[4], lhs[6]]; // v_e
        let lhs_odd = [lhs[1], lhs[3], lhs[5], lhs[7]]; // v_o
        let mut rhs_even = [rhs[0], rhs[2], rhs[4], rhs[6]]; // u_e
        let rhs_odd = [rhs[1], rhs[3], rhs[5], rhs[7]]; // u_o

        let mut prod_even = Self::signed_conv4(&lhs_even, &rhs_even); // v_e(x)u_e(x) mod x^4 + 1
        let prod_odd = Self::signed_conv4(&lhs_odd, &rhs_odd); // v_o(x)u_o(x) mod x^4 + 1

        // Add the two halves together, storing the result in lhs_even/rhs_even.
        add_mut(&mut lhs_even, &lhs_odd); // v_e + v_o
        add_mut(&mut rhs_even, &rhs_odd); // u_e + u_o

        let mut prod_mix = Self::signed_conv4(&lhs_even, &rhs_even); // (v_e(x) + v_o(x))(u_e(x) + u_o(x))
        sub_mut(&mut prod_mix, &prod_even);
        sub_mut(&mut prod_mix, &prod_odd); // (v_e(x) + v_o(x))(u_e(x) + u_o(x)) - v_e(x)u_e(x) - v_o(x)u_o(x)

        add_mut(&mut prod_even[1..], &prod_odd[..(HALF - 1)]);
        prod_even[0] -= prod_odd[HALF - 1]; // v_e(x)u_e(x) + xv_o(x)u_o(x) mod x^4 + 1

        [
            prod_even[0],
            prod_mix[0],
            prod_even[1],
            prod_mix[1],
            prod_even[2],
            prod_mix[2],
            prod_even[3],
            prod_mix[3],
        ] // Intertwining the result. Again this is some annoying data fiddiling. Must be a way to avoid some of this.
    }

    /////////////////////////////////////////////////////////////////////////////////////////////////////
    // Length 16

    /// Compute the convolution of 2 vectors of length 8.
    /// output(x) = lhs(x)rhs(x) mod x^8 - 1  <=>  output = lhs * rhs
    /// Use the FFT Trick to split into a convolution of length 4 and a signed convolution of length 4.
    #[inline]
    fn conv16<T: NonCanonicalPrimeField32>(lhs: [T; 16], rhs: [T; 16], output: &mut [T]) {
        const N: usize = 16;
        const HALF: usize = N / 2;

        // Compute lhs(x) mod x^8 - 1, lhs(x) mod x^8 + 1
        let (lhs_p, lhs_m) = split_add_sub(lhs);

        // rhs will always be constant. Not sure how to tell the compiler this though.
        // Compute rhs(x) mod x^8 - 1, rhs(x) mod x^8 + 1
        let (rhs_p, rhs_m) = split_add_sub(rhs);

        let (left, right) = output.split_at_mut(HALF);
        left.clone_from_slice(&Self::signed_conv8(&lhs_m, &rhs_m)); // left = w_1 = lhs*rhs mod x^8 + 1
        Self::conv8(lhs_p, rhs_p, right); // right = w_0 = lhs*rhs mod x^8 - 1
        for i in 0..HALF {
            left[i] += right[i]; // w_0 + w_1
            left[i] >>= 1; // (w_0 + w_1)/2
            right[i] -= left[i]; // (w_0 - w_1)/2
        }
    }

    /// Compute the signed convolution of 2 vectors of length 16.
    /// output(x) = lhs(x)rhs(x) mod x^16 + 1
    /// Use the Karatsuba Method to split into 3 degree 3 polynomial multiplications.
    #[inline]
    fn signed_conv16<T: NonCanonicalPrimeField32>(lhs: &[T; 16], rhs: &[T; 16]) -> [T; 16] {
        const N: usize = 16;
        const HALF: usize = N / 2;

        // The algorithm is relatively simple:
        // v(x)u(x) mod x^16 + 1 = (v_e(x^2) + xv_o(x^2))(u_e(x^2) + xu_o(x^2)) mod x^16 + 1
        //          = v_e(x^2)u_e(x^2) + x^2 v_o(x^2)u_o(x^2) + x((v_e(x^2) + v_o(x^2))(u_e(x^2) + u_o(x^2)) - v_e(x^2)u_e(x^2) - v_o(x^2)u_o(x^2))

        // Now computing v_e(x^2)u_e(x^2) mod x^16 + 1 is equivalent to computing v_e(x)u_e(x) mod x^8 + 1 and similarly for the other products.

        // Clearly there should be a cleaner way to get this decomposition but everything I've tried has been slower.
        // Also seems like we are doing quite a bit of data fiddiling. Would be nice to avoid this.
        let mut lhs_even = [
            lhs[0], lhs[2], lhs[4], lhs[6], lhs[8], lhs[10], lhs[12], lhs[14], // v_e
        ];
        let lhs_odd = [
            lhs[1], lhs[3], lhs[5], lhs[7], lhs[9], lhs[11], lhs[13], lhs[15], // v_o
        ];
        let mut rhs_even = [
            rhs[0], rhs[2], rhs[4], rhs[6], rhs[8], rhs[10], rhs[12], rhs[14], // u_e
        ];
        let rhs_odd = [
            rhs[1], rhs[3], rhs[5], rhs[7], rhs[9], rhs[11], rhs[13], rhs[15], // u_o
        ];

        let mut prod_even = Self::signed_conv8(&lhs_even, &rhs_even); // v_e(x)u_e(x) mod x^8 + 1
        let prod_odd = Self::signed_conv8(&lhs_odd, &rhs_odd); // v_o(x)u_o(x) mod x^8 + 1

        // Add the two halves together, storing the result in lhs_even/rhs_even.
        add_mut(&mut lhs_even, &lhs_odd); // v_e + v_o
        add_mut(&mut rhs_even, &rhs_odd); // u_e + u_o

        let mut prod_mix = Self::signed_conv8(&lhs_even, &rhs_even); // (v_e(x) + v_o(x))(u_e(x) + u_o(x))
        sub_mut(&mut prod_mix, &prod_even);
        sub_mut(&mut prod_mix, &prod_odd); // (v_e(x) + v_o(x))(u_e(x) + u_o(x)) - v_e(x)u_e(x) - v_o(x)u_o(x)

        add_mut(&mut prod_even[1..], &prod_odd[..(HALF - 1)]);
        prod_even[0] -= prod_odd[HALF - 1]; // v_e(x)u_e(x) + xv_o(x)u_o(x) mod x^8 + 1

        [
            prod_even[0],
            prod_mix[0],
            prod_even[1],
            prod_mix[1],
            prod_even[2],
            prod_mix[2],
            prod_even[3],
            prod_mix[3],
            prod_even[4],
            prod_mix[4],
            prod_even[5],
            prod_mix[5],
            prod_even[6],
            prod_mix[6],
            prod_even[7],
            prod_mix[7],
        ] // Intertwining the result. Again this is some annoying data fiddiling. Must be a way to avoid some of this.
    }

    /////////////////////////////////////////////////////////////////////////////////////////////////////
    // Length 32

    /// Compute the convolution of 2 vectors of length 32.
    /// output(x) = lhs(x)rhs(x) mod x^32 - 1  <=>  output = lhs * rhs
    /// Use the FFT Trick to split into a convolution of length 16 and a signed convolution of length 16.
    #[inline]
    fn conv32<T: NonCanonicalPrimeField32>(lhs: [T; 32], rhs: [T; 32], output: &mut [T]) {
        const N: usize = 32;
        const HALF: usize = N / 2;

        // Compute lhs(x) mod x^16 - 1, lhs(x) mod x^16 + 1
        let (lhs_p, lhs_m) = split_add_sub(lhs);

        // rhs will always be constant. Not sure how to tell the compiler this though.
        // Compute rhs(x) mod x^16 - 1, rhs(x) mod x^16 + 1
        let (rhs_p, rhs_m) = split_add_sub(rhs);

        let (left, right) = output.split_at_mut(HALF);
        left.clone_from_slice(&Self::signed_conv16(&lhs_m, &rhs_m)); // left = w_1 = lhs*rhs mod x^16 + 1
        Self::conv16(lhs_p, rhs_p, right); // right = w_0 = lhs*rhs mod x^16 - 1
        for i in 0..HALF {
            left[i] += right[i]; // w_0 + w_1
            left[i] >>= 1; // (w_0 + w_1)/2
            right[i] -= left[i]; // (w_0 - w_1)/2
        }
    }

    /// Compute the signed convolution of 2 vectors of length 16.
    /// output(x) = lhs(x)rhs(x) mod x^16 + 1
    /// Use the Karatsuba Method to split into 3 degree 3 polynomial multiplications.
    #[inline]
    fn signed_conv32<T: NonCanonicalPrimeField32>(lhs: &[T; 32], rhs: &[T; 32]) -> [T; 32] {
        const N: usize = 32;
        const HALF: usize = N / 2;

        // The algorithm is simple:
        // v(x)u(x) mod x^32 + 1 = (v_l(x) + x^4v_h(x))(u_l(x) + x^4u_h(x)) mod x^32 + 1
        //          = v_l(x)u_l(x) - v_h(x)u_h(x) + x^4((v_l(x) + v_h(x))(u_l(x) + u_h(x)) - v_l(x)u_l(x) - v_h(x)u_h(x))

        // Now computing v_e(x^2)u_e(x^2) mod x^32 + 1 is equivalent to computing v_e(x)u_e(x) mod x^16 + 1 and similarly for the other products.

        // Clearly there should be a cleaner way to get this decomposition but everything I've tried has been slower.
        // Also seems like we are doing quite a bit of data fiddiling. Would be nice to avoid this.
        let mut lhs_even = [
            lhs[0], lhs[2], lhs[4], lhs[6], lhs[8], lhs[10], lhs[12], lhs[14], lhs[16],
            lhs[18], // v_e
            lhs[20], lhs[22], lhs[24], lhs[26], lhs[28], lhs[30],
        ];
        let lhs_odd = [
            lhs[1], lhs[3], lhs[5], lhs[7], lhs[9], lhs[11], lhs[13], lhs[15], lhs[17],
            lhs[19], // v_o
            lhs[21], lhs[23], lhs[25], lhs[27], lhs[29], lhs[31],
        ];
        let mut rhs_even = [
            rhs[0], rhs[2], rhs[4], rhs[6], rhs[8], rhs[10], rhs[12], rhs[14], rhs[16],
            rhs[18], // u_e
            rhs[20], rhs[22], rhs[24], rhs[26], rhs[28], rhs[30],
        ];
        let rhs_odd = [
            rhs[1], rhs[3], rhs[5], rhs[7], rhs[9], rhs[11], rhs[13], rhs[15], rhs[17],
            rhs[19], // u_o
            rhs[21], rhs[23], rhs[25], rhs[27], rhs[29], rhs[31],
        ];

        let mut prod_even = Self::signed_conv16(&lhs_even, &rhs_even); // v_e(x)u_e(x) mod x^16 + 1
        let prod_odd = Self::signed_conv16(&lhs_odd, &rhs_odd); // v_o(x)u_o(x) mod x^16 + 1

        // Add the two halves together, storing the result in lhs_even/rhs_even.
        add_mut(&mut lhs_even, &lhs_odd); // v_e + v_o
        add_mut(&mut rhs_even, &rhs_odd); // u_e + u_o

        let mut prod_mix = Self::signed_conv16(&lhs_even, &rhs_even); // (v_e(x) + v_o(x))(u_e(x) + u_o(x)) mod x^16 + 1
        sub_mut(&mut prod_mix, &prod_even);
        sub_mut(&mut prod_mix, &prod_odd); // (v_e(x) + v_o(x))(u_e(x) + u_o(x)) - v_e(x)u_e(x) - v_o(x)u_o(x)

        add_mut(&mut prod_even[1..], &prod_odd[..(HALF - 1)]);
        prod_even[0] -= prod_odd[HALF - 1]; // v_e(x)u_e(x) + xv_o(x)u_o(x) mod x^16 + 1

        [
            prod_even[0],
            prod_mix[0],
            prod_even[1],
            prod_mix[1],
            prod_even[2],
            prod_mix[2],
            prod_even[3],
            prod_mix[3],
            prod_even[4],
            prod_mix[4],
            prod_even[5],
            prod_mix[5],
            prod_even[6],
            prod_mix[6],
            prod_even[7],
            prod_mix[7],
            prod_even[8],
            prod_mix[8],
            prod_even[9],
            prod_mix[9],
            prod_even[10],
            prod_mix[10],
            prod_even[11],
            prod_mix[11],
            prod_even[12],
            prod_mix[12],
            prod_even[13],
            prod_mix[13],
            prod_even[14],
            prod_mix[14],
            prod_even[15],
            prod_mix[15],
        ] // Intertwining the result. Again this is some annoying data fiddiling. Must be a way to avoid some of this.
    }

    /////////////////////////////////////////////////////////////////////////////////////////////////////
    // Length 64

    /// Compute the convolution of 2 vectors of length 64.
    /// output(x) = lhs(x)rhs(x) mod x^64 - 1  <=>  output = lhs * rhs
    /// Use the FFT Trick to split into a convolution of length 32 and a signed convolution of length 32.
    #[inline]
    fn conv64<T: NonCanonicalPrimeField32>(lhs: [T; 64], rhs: [T; 64], output: &mut [T]) {
        const N: usize = 64;
        const HALF: usize = N / 2;

        let (lhs_left, lhs_right) = lhs.split_at(HALF);

        let lhs_p = add_vec(lhs_left, lhs_right); // lhs(x) mod x^32 - 1
        let lhs_m = sub_vec(lhs_left, lhs_right); // lhs(x) mod x^32 + 1

        // rhs will always be constant. Not sure how to tell the compiler this though.
        let (rhs_left, rhs_right) = rhs.split_at(HALF);

        let rhs_p = add_vec(rhs_left, rhs_right); // rhs(x) mod x^32 - 1
        let rhs_m = sub_vec(rhs_left, rhs_right); // rhs(x) mod x^32 + 1

        let (left, right) = output.split_at_mut(HALF);
        left.clone_from_slice(&Self::signed_conv32(&lhs_m, &rhs_m)); // left = w_1 = lhs*rhs mod x^32 + 1
        Self::conv32(lhs_p, rhs_p, right); // right = w_0 = lhs*rhs mod x^32 - 1
        for i in 0..HALF {
            left[i] += right[i]; // w_0 + w_1
            left[i] >>= 1; // (w_0 + w_1)/2
            right[i] -= left[i]; // (w_0 - w_1)/2
        }
    }
}

// We want to compute a convolution lhs * rhs of 31 bit field elements.
// For any sensible algorithm we use, the size of elements computed in intermediate steps is bounded by
// Sum(lhs) * Sum(rhs) ~ N^2 2**62 for a convolution of size N and generic field elements.
// Thus this will overflow an i64 and so we need to we need to include reductions and/or pass to i128's breifly.
struct LargeConvolution;

// We proivide a quick proof that our strategy for LargeConvolution will produce the correct result.
// We focus on the N = 64 case as any case involving smaller N will be strictly easier.

// Entries start as field elements, so i32's.
// In each reduction step we add or subtract some entries so by the time we have gotten to size 4 convs the entries are i36's.
// The size 4 convs involve products and adding together 4 things so our entries become i74's.
// Now we apply a reduction step which reduces i80's -> i50's.
// Moving back up, n each karatsuba step we have to compute a difference of 3 elements.
// We have 3 karatsuba steps making the entries size i55's.
// CRT steps don't increase the size due to the division by 2 so our entries will remain i55's which will not overflow.

impl Convolution for LargeConvolution {
   /// Compute the convolution of two vectors of length 4.
   /// output(x) = lhs(x)rhs(x) mod x^4 - 1 in Fp[X]
   /// Coefficients will be non canonical representatives.
    fn conv4<T: NonCanonicalPrimeField32>(lhs: [T; 4], rhs: [T; 4], output: &mut [T]) {
        // Even at this small size, doing the FFT decomposition seems to produce shorter compiled code using godbolt.
        // In particular testing the code produced for conv8.

        let lhs_p = [lhs[0] + lhs[2], lhs[1] + lhs[3]]; // v_0(x)
        let lhs_m = [lhs[0] - lhs[2], lhs[1] - lhs[3]]; // v_1(x)

        let rhs_p = [rhs[0] + rhs[2], rhs[1] + rhs[3]]; // u_0(x)
        let rhs_m = [rhs[0] - rhs[2], rhs[1] - rhs[3]]; // u_1(x)

        output[0] = T::from_small_i128(lhs_m[0] * rhs_m[0] - lhs_m[1] * rhs_m[1]);
        output[1] = T::from_small_i128(lhs_m[0] * rhs_m[1] + lhs_m[1] * rhs_m[0]); // output[0, 1] = w_1 = v_1(x)u_1(x) mod x^2 + 1
        output[2] = T::from_small_i128(lhs_p[0] * rhs_p[0] + lhs_p[1] * rhs_p[1]);
        output[3] = T::from_small_i128(lhs_p[0] * rhs_p[1] + lhs_p[1] * rhs_p[0]); // output[2, 3] = w_0 = v_0(x)u_0(x) mod x^2 - 1

        output[0] += output[2];
        output[1] += output[3]; // output[0, 1] = w_1 + w_0

        output[0] >>= 1;
        output[1] >>= 1; // output[0, 1] = (w_1 + w_0)/2)

        output[2] -= output[0];
        output[3] -= output[1]; // output[2, 3] = w_0 - (w_1 + w_0)/2) = (w_0 - w_1)/2
    }

    fn signed_conv4_mut<T: NonCanonicalPrimeField32>(lhs: &[T; 4], rhs: &[T; 4], output: &mut [T]) {
        let rhs_rev = [rhs[3], rhs[2], rhs[1], rhs[0]];

        output[0] = T::from_small_i128(
            (lhs[0] * rhs_rev[3]) - dot_i128(&lhs[1..], &rhs_rev[..3]),
        ); // v_0u_0 - (v_1u_3 + v_2u_2 + v_3u_1)
        output[1] = T::from_small_i128(
            dot_i128(&lhs[..2], &rhs_rev[2..]) - dot_i128(&lhs[2..], &rhs_rev[..2]),
        ); // v_0u_1 + v_1u_0 - (v_2u_3 + v_2u_3)
        output[2] = T::from_small_i128(
            dot_i128(&lhs[..3], &rhs_rev[1..]) - (lhs[3] * rhs_rev[0]),
        ); // v_0u_2 + v_1u_1 + v_2u_0 - v_3u_3
        output[3] = T::from_small_i128(dot_i128(lhs, &rhs_rev)); // v_0u_3 + v_1u_2 + v_2u_1 + v_3u_0

        // This might not be the best way to compute this.
        // Another approach is to define
        // [rhs[0], -rhs[3], -rhs[2], -rhs[1]]
        // [rhs[1], rhs[0], -rhs[3], -rhs[2]]
        // [rhs[2], rhs[1], rhs[0], -rhs[3]]
        // [rhs[3], rhs[2], rhs[1], rhs[0]]
        // And then take dot products.
        // Might also be other methods in particular we might be able to pick MDS matrices to make this simpler.
    }

    fn signed_conv4<T: NonCanonicalPrimeField32>(lhs: &[T; 4], rhs: &[T; 4]) -> [T; 4] {
        let mut output = [T::zero(); 4];

        Self::signed_conv4_mut(lhs, rhs, &mut output);

        output
    }
}

// If we can add the assumption that Sum(lhs) < 2**20 then
// Sum(lhs)*Sum(rhs) < N * 2**{51} and so, for small N we can work with i64's and ignore overflow.
struct SmallConvolution;

impl Convolution for SmallConvolution {
    /// Compute the convolution of two vectors of length 4. We assume we can ignore overflow so
    /// output(x) = lhs(x)rhs(x) mod x^4 - 1 in Z[X]
    #[inline]
    fn conv4<T: NonCanonicalPrimeField32>(lhs: [T; 4], rhs: [T; 4], output: &mut [T]) {
        // Even at this small size, doing the FFT decomposition seems to produce shorter compiled code using godbolt.
        // In particular testing the code produced for conv8.
        let lhs_p = [lhs[0] + lhs[2], lhs[1] + lhs[3]]; // v_0(x)
        let lhs_m = [lhs[0] - lhs[2], lhs[1] - lhs[3]]; // v_1(x)

        let rhs_p = [rhs[0] + rhs[2], rhs[1] + rhs[3]]; // u_0(x)
        let rhs_m = [rhs[0] - rhs[2], rhs[1] - rhs[3]]; // u_1(x)

        output[0] = T::mul_small(lhs_m[0], rhs_m[0]) - T::mul_small(lhs_m[1], rhs_m[1]);
        output[1] = T::mul_small(lhs_m[0], rhs_m[1]) + T::mul_small(lhs_m[1], rhs_m[0]); // output[0, 1] = w_1 = v_1(x)u_1(x) mod x^2 + 1
        output[2] = T::mul_small(lhs_p[0], rhs_p[0]) + T::mul_small(lhs_p[1], rhs_p[1]);
        output[3] = T::mul_small(lhs_p[0], rhs_p[1]) + T::mul_small(lhs_p[1], rhs_p[0]);

        output[0] += output[2];
        output[1] += output[3]; // output[0, 1] = w_1 + w_0

        output[0] >>= 1;
        output[1] >>= 1; // output[0, 1] = (w_1 + w_0)/2)

        output[2] -= output[0];
        output[3] -= output[1]; // output[2, 3] = w_0 - (w_1 + w_0)/2) = (w_0 - w_1)/2
    }

    /// Compute the signed convolution of two vectors of length 4.
    /// output(x) = lhs(x)rhs(x) mod x^4 + 1
    #[inline]
    fn signed_conv4_mut<T: NonCanonicalPrimeField32>(lhs: &[T; 4], rhs: &[T; 4], output: &mut [T]) {

        let rhs_rev = [rhs[3], rhs[2], rhs[1], rhs[0]];

        output[0] = T::mul_small(lhs[0], rhs[0]) - dot_i64(&lhs[1..], &rhs_rev[..3]); // v_0u_0 - (v_1u_3 + v_2u_2 + v_3u_1)
        output[1] = dot_i64(&lhs[..2], &rhs_rev[2..]) - dot_i64(&lhs[2..], &rhs_rev[..2]); // v_0u_1 + v_1u_0 - (v_2u_3 + v_2u_3)
        output[2] = dot_i64(&lhs[..3], &rhs_rev[1..]) - T::mul_small(lhs[3], rhs[3]); // v_0u_2 + v_1u_1 + v_2u_0 - v_3u_3
        output[3] = dot_i64(lhs, &rhs_rev); // v_0u_3 + v_1u_2 + v_2u_1 + v_3u_0

        // This might not be the best way to compute this.
        // Another approach is to define
        // [rhs[0], -rhs[3], -rhs[2], -rhs[1]]
        // [rhs[1], rhs[0], -rhs[3], -rhs[2]]
        // [rhs[2], rhs[1], rhs[0], -rhs[3]]
        // [rhs[3], rhs[2], rhs[1], rhs[0]]
        // And then take dot products.
        // Might also be other methods in particular we might be able to pick MDS matrices to make this simpler.
    }

    /// Compute the signed convolution of two vectors of length 4.
    /// output(x) = lhs(x)rhs(x) mod x^4 + 1
    #[inline]
    fn signed_conv4<T: NonCanonicalPrimeField32>(lhs: &[T; 4], rhs: &[T; 4]) -> [T; 4] {
        let mut output = [T::zero(); 4];

        Self::signed_conv4_mut(lhs, rhs, &mut output);

        output
    }
}




// It will be handy for functions to be able to handle entries which are a combination of simple integer types
// In particular u64's, i64's, u128's and i128's so we make a general trait type here.
trait SimpleInteger:
    Sized
    + Default
    + Copy
    + Add<Output = Self>
    + AddAssign
    + Sub<Output = Self>
    + SubAssign
    + Mul<Output = Self>
    + MulAssign
    + ShrAssign<usize>
    + Shr<usize, Output = Self>
    + ShlAssign<usize>
    + Shl<usize, Output = Self>
{
}

impl<T> SimpleInteger for T where
    T: Sized
        + Default
        + Copy
        + Add<Output = Self>
        + AddAssign
        + Sub<Output = Self>
        + SubAssign
        + Mul<Output = Self>
        + MulAssign
        + ShrAssign<usize>
        + Shr<usize, Output = Self>
        + ShlAssign<usize>
        + Shl<usize, Output = Self>
{
}

impl<T> IntegerLike for T where T: SimpleInteger {}

/// Computes the convolution of input and MATRIX_CIRC_MDS_8_SML.
/// Input must be an array of field elements of length 8.
/// Only works with Mersenne31 and Babybear31
pub fn apply_circulant_8_karat<Base: PrimeField32, F: Canonicalize<Base>>(input: [Base; 8]) -> [Base; 8] {
    // Flip MATRIX_CIRC_MDS_8_SML to get the first column of the circulant matrix.
    // This is constant of course but unclear how to tell the compiler that right now.
    // Hopefully it works it out.
    let matrix_circ_mds_8_sml_i64: [F; 8] = row_to_col(MATRIX_CIRC_MDS_8_SML).map(F::from_i64);

    // The numbers we will encounter through our algorithm are (roughly) bounded by
    // SUM(input.as_canonical_u64()) * SUM(MATRIX_CIRC_MDS_8_SML)
    // <= (8 * 2**31) * 33 < 2**40 << 2**63
    // Hence we can work with i64's with garuntees of no overflow occuring.
    let input_non_canonical = input.map(F::from_canonical);

    // Compute the convolution.
    // Currently might? not taking full advantage of MATRIX_CIRC_MDS_8_SML_I64 being constant.
    let mut output: [F; 8] = [F::zero(); 8];
    SmallConvolution::conv8(input_non_canonical, matrix_circ_mds_8_sml_i64, &mut output);

    // Whilst some intermediate steps may be negative, as we started with 2 positive vectors
    // The output will always be positive and is bounded by 2**40.
    // output.map(|x| F::from_wrapped_u64(x as u64))
    output.map(F::to_canonical_u_small)
}

pub fn apply_circulant_12_karat<F: PrimeField64>(input: [F; 12]) -> [F; 12] {
    // Flip MATRIX_CIRC_MDS_12_SML to get the first column of the circulant matrix.
    const MATRIX_CIRC_MDS_12_SML_I64: [i64; 12] = row_to_col(MATRIX_CIRC_MDS_12_SML);

    // The numbers we will encounter through our algorithm are bounded by
    // SUM(input.as_canonical_u64()) * SUM(MATRIX_CIRC_MDS_8_SML) <= (12 * 2**31) * 33 < 2**40 << 2**63
    // Hence we can work with i64's with garuntees of no overflow occuring.
    let input_i64 = input.map(|x| x.as_canonical_u64() as i64);

    // Compute the convolution.
    // Currently might? not taking full advantage of MATRIX_CIRC_MDS_8_SML_I64 being constant.
    let output = conv12(input_i64, MATRIX_CIRC_MDS_12_SML_I64);

    // Whilst some intermediate steps may be negative, as we started with 2 positive vectors
    // The output will always be positive and is bounded by 2**40.
    // output.map(|x| F::from_wrapped_u64(x as u64))
    output.map(red_u62_m31)
}

/// Computes the convolution of input and MATRIX_CIRC_MDS_16_SML.
/// Input must be an array of field elements of length 16.
/// Only works with Mersenne31 and Babybear31
pub fn apply_circulant_16_karat<Base: PrimeField32, F: Canonicalize<Base>>(input: [Base; 16]) -> [Base; 16] {
    // Flip MATRIX_CIRC_MDS_16_SML_I64 to get the first column of the circulant matrix.
    // This is constant of course but unclear how to tell the compiler that right now.
    // Hopefully it works it out.
    let matrix_circ_mds_16_sml_i64: [F; 16] = row_to_col(MATRIX_CIRC_MDS_16_SML).map(F::from_i64);

    // The numbers we will encounter through our algorithm are (roughly) bounded by
    // SUM(input.as_canonical_u64()) * SUM(MATRIX_CIRC_MDS_8_SML)
    // <= (8 * 2**31) * 33 < 2**40 << 2**63
    // Hence we can work with i64's with garuntees of no overflow occuring.
    let input_non_canonical = input.map(F::from_canonical);

    // Compute the convolution.
    // Currently might? not taking full advantage of MATRIX_CIRC_MDS_8_SML_I64 being constant.
    let mut output: [F; 16] = [F::zero(); 16];
    SmallConvolution::conv16(input_non_canonical, matrix_circ_mds_16_sml_i64, &mut output);

    // Whilst some intermediate steps may be negative, as we started with 2 positive vectors
    // The output will always be positive and is bounded by 2**40.
    // output.map(|x| F::from_wrapped_u64(x as u64))
    output.map(F::to_canonical_u_small)
}

/// Computes the convolution of input and MATRIX_CIRC_MDS_32_MERSENNE31.
/// Input must be an array of Mersenne31 field elements of length 32.
pub fn apply_circulant_32_karat<Base: PrimeField32, F: Canonicalize<Base>>(input: [Base; 32]) -> [Base; 32] {
    // Flip MATRIX_CIRC_MDS_32_MERSENNE31 to get the first column of the circulant matrix.
    let matrix_circ_mds_32_m31_i128: [F; 32] = row_to_col(MATRIX_CIRC_MDS_32_MERSENNE31).map(F::from_i64);

    // The numbers we will encounter through our algorithm are > 2**64 as
    // SUM(input.as_canonical_u64()) * SUM(MATRIX_CIRC_MDS_32_MERSENNE31) <= (32 * 2**31)**2 < 2**72.
    // Hence we need to do some intermediate reductions.
    let input_non_canonical = input.map(F::from_canonical);

    // Compute the convolution.
    // Currently might? not taking full advantage of MATRIX_CIRC_MDS_8_SML_I64 being constant.
    let mut output: [F; 32] = [F::zero(); 32];
    LargeConvolution::conv32(input_non_canonical, matrix_circ_mds_32_m31_i128, &mut output);

    // x is an i49 => (P << 20) + x is positive.
    output.map(F::to_canonical_i_small)
}

/// Computes the convolution of input and MATRIX_CIRC_MDS_64_MERSENNE31.
/// Input must be an array of Mersenne31 field elements of length 64.
pub fn apply_circulant_64_karat<Base: PrimeField32, F: Canonicalize<Base>>(input: [Base; 64]) -> [Base; 64] {
    // Flip MATRIX_CIRC_MDS_64_MERSENNE31 to get the first column of the circulant matrix.
    let matrix_circ_mds_64_m31_i128: [F; 64] = row_to_col(MATRIX_CIRC_MDS_64_MERSENNE31).map(F::from_i64);

    // The numbers we will encounter through our algorithm are > 2**64 as
    // SUM(input.as_canonical_u64()) * SUM(MATRIX_CIRC_MDS_64_MERSENNE31) < (64 * 2**31)**2 < 2**74 << 2**127
    // Hence we need to do some intermediate reductions.
    let input_i128 = input.map(F::from_canonical);

    // Compute the convolution.
    // Currently might? not taking full advantage of MATRIX_CIRC_MDS_8_SML_I64 being constant.
    let mut output: [F; 64] = [F::zero(); 64];
    LargeConvolution::conv64(input_i128, matrix_circ_mds_64_m31_i128, &mut output);

    // x is an i49 => (P << 20) + x is positive.
    output.map(F::to_canonical_i_small)
}

// Let M be a circulant matrix with first column vec_col and first row vec_row. Then M.u is the convolution of vec_col and u.
// The vectors given here are the first rows of the respective circulant matrices NOT the first colums.
// Hence in order to do convolutions we need to compute the first column which is given by
// vec_col = [vec_row[0], vec_row[n - 1], vec_row[n - 2], ..., vec_row[1]]
#[inline]
const fn row_to_col<T: SimpleInteger, const N: usize>(row: [T; N]) -> [T; N] {
    let mut col = [row[0]; N];
    let mut i = 1;
    loop {
        if i == N {
            break;
        }
        col[i] = row[N - i];
        i += 1
    }
    col
}

/// Reduces an i64 in the range 0 <= x < 2^62 to its (almost) canonical representative mod 2^31 - 1.
/// Not technically canonical as we allow P as an output.
#[inline]
fn red_u62_m31<F: PrimeField64>(input: i64) -> F {
    // Morally, our value is a u62 not a i64 as the top 2 bits are garunteed to be 0.
    debug_assert!((0..(1 << 62)).contains(&input));

    let low_bits = F::from_canonical_u32((input & P) as u32); // Get the bottom 31 bits, 0 <= low_bits <= P
    let high_bits = F::from_canonical_u32((input >> 31) as u32); // Get the top 31 bits, 0 <= high_bits <= P.

    low_bits + high_bits
}

/// Reduces an i64 in the range 0 <= x < 2^93 to its (almost) canonical representative mod 2^31 - 1.
/// Not technically canonical as we allow P as an output.
#[inline]
fn _red_u93_m31<F: PrimeField64>(input: i128) -> F {
    // Not used currently. Useful if we full embrace i128's in the larger convolutions.
    // Morally, our value is a u93 not a i128 as the top bits are garunteed to be 0.
    debug_assert!((0..(1 << 93)).contains(&input));

    let low_bits = F::from_canonical_u32((input & (P as i128)) as u32); // Get the bottom 31 bits, 0 <= low_bits <= P
    let mid_bits = F::from_canonical_u32(((input >> 31) & (P as i128)) as u32); // Get the mid 31 bits, 0 <= high_bits <= P.
    let high_bits = F::from_canonical_u32((input >> 62) as u32); // Get the top 31 bits, 0 <= high_bits <= P.

    low_bits + mid_bits + high_bits
}

// Takes the dot product of two vectors.
#[inline]
fn dot<T: SimpleInteger>(lhs: &[T], rhs: &[T]) -> T {
    let n = lhs.len();
    let mut sum = lhs[0] * rhs[0];
    for i in 1..n {
        sum += lhs[i] * rhs[i];
    }
    sum
}

// Given a vector v \in F^N, let v(x) \in F[X] denote the polynomial v_0 + v_1 x + ... + v_{N - 1} x^{N - 1}.
// Then w is equal to the convolution v * u if and only if w(x) = v(x)u(x) mod x^N - 1.
// Additionally, define the signed convolution by w(x) = v(x)u(x) mod x^N + 1.

// Using the chinese remainder theorem we can compute w(x) from:
//                      w_0 = v(x)u(x) mod x^{N/2} - 1
//                      w_1 = v(x)u(x) mod x^{N/2} + 1
// Via:
//                      w(x) = 1/2 (w_0(x) + w_1(x)) + x^{N/2}/2 (w_0(x) - w_1(x))

// To compute w_0 and w_1 we first compute
//                  v_0(x) = v(x) mod x^{N/2} - 1
//                  v_1(x) = v(x) mod x^{N/2} + 1
//                  u_0(x) = v(x) mod x^{N/2} - 1
//                  u_1(x) = v(x) mod x^{N/2} + 1

// Now w_0 is the convolution of v_0 and u_0 so this can be applied recursively.
// For w_1 we compute the signed convolution v_1(x)u_1(x) mod x^{N/2} + 1 using Karatsuba.

// There are 2 possible approaches to this karatsuba which mirror the DIT vs DIF approaches to FFT's.

// Option 1: left/right decomposition.
// Write v = (v_l, v_r) so that v(x) = (v_l(x) + x^{N/2}v_r(x)).
// Then v(x)u(x) mod x^N + 1 = (v_l(x)u_l(x) - v_r(x)u_r(x)) + x^{N/2}((v_l(x) + v_r(x))(u_l(x) + u_r(x)) - (v_l(x)u_l(x) + v_r(x)u_r(x))) mod X^N + 1

// As v_l(x), v_r(x), u_l(x), u_r(x) all have degree < N/2 no product will have degree > N - 1.
// The only place we need to deal with the mod operation is after the multipication by x^{N/2} and this is easy to do.
// Thus this reduces the problem to 3 polynomial multiplications of size N/2 and we can use the standard karatsuba for this.

// Option 2: even/odd decomposition.
// Define the even v_e and odd v_o parts so that v(x) = (v_e(x^2) + xv_o(x^2)).
// Then v(x)u(x) = (v_e(x^2)u_e(x^2) + x^2 v_o(x^2)u_o(x^2)) + x ((v_e(x^2) + v_o(x^2))(u_e(x^2) + u_o(x^2)) - (v_e(x^2)u_e(x^2) + v_o(x^2)u_o(x^2))
// This reduces the problem to 3 signed convolutions of size N/2 and we can do this recursively.

// Option 2: seems to involve less total operations and so should be faster hence it is the once we have implmented for now.
// The main issue is that we are currently doing quite a bit of data manipulation. (Needing to split vectors into odd and even parts and recombining afterwards)
// Would be good to try and find a way to cut down on this.

// Once we get down to small sizes we use the O(n^2) approach.

/////////////////////////////////////////////////////////////////////////////////////////////////////
// Length 3

/// Compute the convolution of two vectors of length 3.
/// output(x) = lhs(x)rhs(x) mod x^3 - 1
#[inline]
fn conv3<T: SimpleInteger>(lhs: [T; 3], rhs: [T; 3]) -> [T; 3] {
    // This is small enough we just explicitely write down the answer.
    [
        lhs[0] * rhs[0] + lhs[1] * rhs[2] + lhs[2] * rhs[1],
        lhs[0] * rhs[1] + lhs[1] * rhs[0] + lhs[2] * rhs[2],
        lhs[0] * rhs[2] + lhs[1] * rhs[1] + lhs[2] * rhs[0],
    ]
}

/// Compute the signed convolution of two vectors of length 3.
/// output(x) = lhs(x)rhs(x) mod x^3 + 1
#[inline]
fn sign_conv3<T: SimpleInteger>(lhs: &[T; 3], rhs: &[T; 3]) -> [T; 3] {
    // This is small enough we just explicitely write down the answer.
    [
        lhs[0] * rhs[0] - lhs[1] * rhs[2] - lhs[2] * rhs[1],
        lhs[0] * rhs[1] + lhs[1] * rhs[0] - lhs[2] * rhs[2],
        lhs[0] * rhs[2] + lhs[1] * rhs[1] + lhs[2] * rhs[0],
    ]
}

/////////////////////////////////////////////////////////////////////////////////////////////////////
// Length 6

/// Compute the convolution of two vectors of length 6.
/// output(x) = lhs(x)rhs(x) mod x^6 - 1
#[inline]
fn conv6<T: SimpleInteger>(lhs: [T; 6], rhs: [T; 6]) -> [T; 6] {
    // Even at this small size, doing the FFT decomposition seems to produce shorter compiled code using godbolt.
    // In particular testing the code produced for conv12 as that's what we really care about.

    const N: usize = 6;
    const HALF: usize = N / 2;
    let mut output = [T::default(); N];

    // Compute lhs(x) mod x^3 - 1, lhs(x) mod x^3 + 1
    let (lhs_p, lhs_m) = split_add_sub(lhs);

    // rhs will always be constant. Not sure how to tell the compiler this though.
    // Compute rhs(x) mod x^3 - 1, rhs(x) mod x^3 + 1
    let (rhs_p, rhs_m) = split_add_sub(rhs);

    let prod_p = conv3(lhs_p, rhs_p); // prod_p(x) = lhs(x)rhs(x) mod x^4 - 1
    let prod_m = sign_conv3(&lhs_m, &rhs_m); // prod_m(x) = lhs(x)rhs(x) mod x^4 + 1

    output[..HALF].clone_from_slice(&prod_p);
    output[HALF..].clone_from_slice(&prod_p); // output = [prod_p, prod_p]

    add_mut(&mut output[..HALF], &prod_m);
    sub_mut(&mut output[HALF..], &prod_m); // output = [prod_p + prod_m, prod_p - prod_m] = 2 (lhs * rhs)

    // Can maybe do this in place?
    output.map(|x| x >> 1) // output = lhs * rhs
}

/// Compute the signed convolution of two vectors of length 6.
/// output(x) = lhs(x)rhs(x) mod x^6 + 1
#[inline]
fn sign_conv6<T: SimpleInteger>(lhs: &[T; 6], rhs: &[T; 6]) -> [T; 6] {
    let mut output = [T::default(); 6];

    // This might not be the best way to compute this.

    let rhs_rev = [rhs[5], rhs[4], rhs[3], rhs[2], rhs[1], rhs[0]];

    output[0] = lhs[0] * rhs[0] - dot(&lhs[1..], &rhs_rev[..5]);
    output[1] = dot(&lhs[..2], &rhs_rev[4..]) - dot(&lhs[2..], &rhs_rev[..4]);
    output[2] = dot(&lhs[..3], &rhs_rev[3..]) - dot(&lhs[3..], &rhs_rev[..3]);
    output[3] = dot(&lhs[..4], &rhs_rev[2..]) - dot(&lhs[4..], &rhs_rev[..2]);
    output[4] = dot(&lhs[..5], &rhs_rev[1..]) - lhs[5] * rhs[5];
    output[5] = dot(lhs, &rhs_rev);
    output
}

/////////////////////////////////////////////////////////////////////////////////////////////////////
// Length 12

/// Compute the convolution of 2 vectors of length 8.
/// output(x) = lhs(x)rhs(x) mod x^12 - 1  <=>  output = lhs * rhs
/// Use the FFT Trick to split into a convolution of length 6 and a signed convolution of length 6.
#[inline]
fn conv12<T: SimpleInteger>(lhs: [T; 12], rhs: [T; 12]) -> [T; 12] {
    const N: usize = 12;
    const HALF: usize = N / 2;
    let mut output = [T::default(); N];

    // Compute lhs(x) mod x^6 - 1, lhs(x) mod x^6 + 1
    let (lhs_p, lhs_m) = split_add_sub(lhs);

    // rhs will always be constant. Not sure how to tell the compiler this though.
    // Compute rhs(x) mod x^6 - 1, rhs(x) mod x^6 + 1
    let (rhs_p, rhs_m) = split_add_sub(rhs);

    let prod_p = conv6(lhs_p, rhs_p); // prod_p(x) = lhs(x)rhs(x) mod x^6 - 1
    let prod_m = sign_conv6(&lhs_m, &rhs_m); // prod_m(x) = lhs(x)rhs(x) mod x^6 + 1

    output[..HALF].clone_from_slice(&prod_p);
    output[HALF..].clone_from_slice(&prod_p); // output = [prod_p, prod_p]

    add_mut(&mut output[..HALF], &prod_m);
    sub_mut(&mut output[HALF..], &prod_m); // output = [prod_p + prod_m, prod_p - prod_m] = 2 (lhs * rhs)

    // Could also do this in place?
    output.map(|x| x >> 1) // output = (lhs * rhs)
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////

// Generic functions

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////

// Assume that lhs is a vector of u64's and rhs is a vector of field elements.
// This computes M(lhs).rhs where M is the circulant matrix with first row equal to lhs.
// It uses the even-odd decomposition.
/// Currently have not implmented the FFT trick to use signed convolutions.
pub fn apply_circulant_karat_generic_i64<F: PrimeField64, const N: usize>(
    lhs: [F; N],
    rhs: [i64; N],
) -> [F; N] {
    // Convert lhs from field elements to i64's
    let lhs_i64 = lhs.map(|x| x.as_canonical_u64() as i64);

    // We need the vector which is the first column of rhs not the first row.
    let rhs_i64 = row_to_col(rhs);

    let mut output = [0; N];

    conv_karat_generic(&lhs_i64, &rhs_i64, &mut output);

    output.map(|x| F::from_wrapped_u64(x as u64))
}

// We produce a generic implementations. It will likely be faster long term to specialise these.
// Given lhs (v) and rhs (u) compute the convolution of lhs and rhs recursively as smaller convolutions and signed convolutions.
// Currently this only works for n a power of 2.
fn conv_karat_generic<T: SimpleInteger>(lhs: &[T], rhs: &[T], output: &mut [T]) {
    let n = lhs.len();
    match n {
        1 => output[0] = lhs[0] * rhs[0],
        2 => {
            output[0] = lhs[0] * rhs[0] + lhs[1] * rhs[1];
            output[1] = lhs[1] * rhs[0] + lhs[0] * rhs[1];
        }
        4 => conv4_slice(lhs, rhs, output),
        _ => {
            let half = n / 2;

            let (lhs_left, lhs_right) = lhs.split_at(half);

            let mut lhs_p = lhs_left.to_vec();
            let mut lhs_m = lhs_left.to_vec();

            add_mut(&mut lhs_p, lhs_right); // lhs(x) mod x^{n/2} - 1
            sub_mut(&mut lhs_m, lhs_right); // lhs(x) mod x^{n/2} + 1

            // rhs will always be constant. Not sure how to tell the compiler this though.
            let (rhs_left, rhs_right) = rhs.split_at(half);

            let mut rhs_p = rhs_left.to_vec();
            let mut rhs_m = rhs_left.to_vec();

            add_mut(&mut rhs_p, rhs_right); // rhs(x) mod x^{n/2} - 1
            sub_mut(&mut rhs_m, rhs_right); // rhs(x) mod x^{n/2} + 1

            let (left, right) = output.split_at_mut(half);
            signed_conv_karat_generic(&lhs_m, &rhs_m, left); // left = w_1 = lhs*rhs mod x^32 + 1
            conv_karat_generic(&lhs_p, &rhs_p, right); // right = w_0 = lhs*rhs mod x^32 - 1
            for i in 0..half {
                left[i] += right[i]; // w_0 + w_1
                left[i] >>= 1; // (w_0 + w_1)/2
                right[i] -= left[i]; // (w_0 - w_1)/2
            }
        }
    }
}

/// Compute the convolution of two vectors of length 4.
/// output(x) = lhs(x)rhs(x) mod x^4 - 1
#[inline]
fn conv4_slice<T: SimpleInteger>(lhs: &[T], rhs: &[T], output: &mut [T]) {
    // lhs.len(), rhs.len() and output.len() should all be equal to 4.
    // Add constant asserts?
    // Even at this small size, doing the FFT decomposition seems to produce shorter compiled code using godbolt.

    let lhs_p = [lhs[0] + lhs[2], lhs[1] + lhs[3]]; // v_0(x)
    let lhs_m = [lhs[0] - lhs[2], lhs[1] - lhs[3]]; // v_1(x)

    // In most cases, rhs will actually be a constant but I'm unsure how to take advantage of this.
    let rhs_p = [rhs[0] + rhs[2], rhs[1] + rhs[3]]; // u_0(x)
    let rhs_m = [rhs[0] - rhs[2], rhs[1] - rhs[3]]; // u_1(x)

    // Might be worth trying to keep everything as a i64 up until this multiplication and
    // only here making things u128's. (Possible that the compiler has already worked this out though.)
    output[0] = lhs_m[0] * rhs_m[0] - lhs_m[1] * rhs_m[1];
    output[1] = lhs_m[0] * rhs_m[1] + lhs_m[1] * rhs_m[0]; // output[0, 1] = w_1 = v_1(x)u_1(x) mod x^2 + 1
    output[2] = lhs_p[0] * rhs_p[0] + lhs_p[1] * rhs_p[1];
    output[3] = lhs_p[0] * rhs_p[1] + lhs_p[1] * rhs_p[0]; // output[2, 3] = w_0 = v_0(x)u_0(x) mod x^2 - 1

    output[0] += output[2];
    output[1] += output[3]; // output[0, 1] = w_1 + w_0

    output[0] >>= 1;
    output[1] >>= 1; // output[0, 1] = (w_1 + w_0)/2)

    output[2] -= output[0];
    output[3] -= output[1]; // output[2, 3] = w_0 - (w_1 + w_0)/2) = (w_0 - w_1)/2
}

/// Compute the signed convolution of two vectors of length 4.
/// output(x) = lhs(x)rhs(x) mod x^4 + 1
/// Same as above but in place.
#[inline]
fn signed_conv4_slice<T: SimpleInteger>(lhs: &[T], rhs: &[T], output: &mut [T]) {
    // lhs.len(), rhs.len() and output.len() should all be equal to 4.
    let rhs_rev = [rhs[3], rhs[2], rhs[1], rhs[0]];

    output[0] = lhs[0] * rhs[0] - dot(&lhs[1..], &rhs_rev[..3]); // v_0u_0 - (v_1u_3 + v_2u_2 + v_3u_1)
    output[1] = dot(&lhs[..2], &rhs_rev[2..]) - dot(&lhs[2..], &rhs_rev[..2]); // v_0u_1 + v_1u_0 - (v_2u_3 + v_2u_3)
    output[2] = dot(&lhs[..3], &rhs_rev[1..]) - lhs[3] * rhs[3]; // v_0u_2 + v_1u_1 + v_2u_0 - v_3u_3
    output[3] = dot(lhs, &rhs_rev); // v_0u_3 + v_1u_2 + v_2u_1 + v_3u_0
}

// Given lhs (v) and rhs (u) compute the signed convolution via the karatsuba method.
fn signed_conv_karat_generic<T: SimpleInteger>(lhs: &[T], rhs: &[T], output: &mut [T]) {
    let n = lhs.len();
    match n {
        1 => output[0] = lhs[0] * rhs[0],
        2 => {
            output[0] = lhs[0] * rhs[0] - lhs[1] * rhs[1];
            output[1] = lhs[1] * rhs[0] + lhs[0] * rhs[1];
        }
        4 => signed_conv4_slice(lhs, rhs, output),
        _ => {
            let half = n / 2;
            let (lhs_even, lhs_odd, lhs_mix) = split_eom(lhs);
            let (rhs_even, rhs_odd, rhs_mix) = split_eom(rhs);

            {
                let (evens, odds) = output.split_at_mut(half);
                let mut extra = vec![T::default(); half];

                signed_conv_karat_generic(&lhs_even, &rhs_even, evens); // v_e(x)u_e(x) mod x^{n/2} + 1
                signed_conv_karat_generic(&lhs_odd, &rhs_odd, &mut extra); // v_o(x)u_o(x) mod x^{n/2} + 1
                signed_conv_karat_generic(&lhs_mix, &rhs_mix, odds); // (v_e(x) + v_o(x))(u_e(x) + u_o(x)) mod x^{n/2} + 1

                sub_mut(odds, evens);
                sub_mut(odds, &extra); // (v_e(x) + v_o(x))(u_e(x) + u_o(x)) - v_e(x)u_e(x) - v_o(x)u_o(x)

                add_mut(&mut evens[1..], &extra[..(half - 1)]);
                evens[0] -= extra[half - 1]; // v_e(x)u_e(x) + xv_o(x)u_o(x) mod x^16 + 1
            }

            rearrange(output);
        }
    }
}

/// Given a vector v, split it into its even, odd and mixed parts.
/// If vec = [v_0, v_1, v_2, ...] then
/// output = ([v_0, v_2, ...], [v_1, v_3, ...], [v_0 + v_1, v_2 + v_3, ...])
#[inline]
fn split_eom<T: SimpleInteger>(vec: &[T]) -> (Vec<T>, Vec<T>, Vec<T>) {
    let half = vec.len() / 2;
    let mut output_even = vec![T::default(); half];
    let mut output_odd = vec![T::default(); half];
    let mut output_mix = vec![T::default(); half];
    for i in 0..half {
        output_even[i] = vec[2 * i];
        output_odd[i] = vec[2 * i + 1];
        output_mix[i] = vec[2 * i] + vec[2 * i + 1];
    }
    (output_even, output_odd, output_mix)
}

/// Given a vector v, interleave it's left and right parts in place
/// if initially vec = [v_0, v_1, ..., v_N, v_{N + 1}, ..., V_{2N - 1}]
/// afterwards vec = [v_0, v_N, v_1, v_{N + 1}, ..., v_{N - 1}, v_{2N - 1}]
#[inline]
fn rearrange<T: SimpleInteger>(vec: &mut [T]) {
    let n = vec.len();
    let half = n / 2;
    let stored_evens = vec[..half].to_vec().clone();
    for i in 0..half {
        vec[2 * i] = stored_evens[i];
        vec[2 * i + 1] = vec[i + half];
    }
}
fn binpow(a: u32, n: u32) -> u32 {
    if n == 0 {
        return 1;
    } else if n % 2 == 1 {
        let half_pow = binpow(a, (n - 1) / 2);
        return half_pow * half_pow * a
    } else {
        let half_pow = binpow(a, n / 2);
        return half_pow * half_pow
    }
}
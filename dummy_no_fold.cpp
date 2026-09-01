#include <cstdlib>
#include <iostream>

int main() {
    const char* limit_str = std::getenv("BENCH_LIMIT");
    const char* bias_str = std::getenv("BENCH_BIAS");
    long long limit = limit_str ? std::atoll(limit_str) : 10000000LL;
    long long bias = bias_str ? std::atoll(bias_str) : 0LL;
    long long total = 0;
    for (long long i = 0; i <= limit; ++i) {
        total += i + bias;
    }
    std::cout << total << std::endl;
    return 0;
}

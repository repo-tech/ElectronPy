#include <iostream>

int main() {
    long long total = 0;
    for (long long i = 0; i <= 10000000; ++i) {
        total += i;
    }
    std::cout << total << std::endl;
    return 0;
}

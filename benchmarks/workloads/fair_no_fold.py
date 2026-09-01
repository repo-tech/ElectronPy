import os


def compute(limit: int, bias: int) -> int:
    total = 0
    for i in range(limit + 1):
        total += i + bias
    return total


if __name__ == "__main__":
    limit = int(os.environ.get("BENCH_LIMIT", "10000000"))
    bias = int(os.environ.get("BENCH_BIAS", "0"))
    print(compute(limit, bias))

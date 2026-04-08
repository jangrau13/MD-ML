#include <iostream>
#include <vector>
#include <random>
#include <cstdint>
#include <iomanip>

#include <Eigen/Core>

#include "utils/Timer.h"
#include "utils/linear_algebra.h"

template <typename T, typename Dist, typename Rng>
void benchSuite(const char* label, Dist& dist, Rng& rng, md_ml::Timer& timer) {
    using MatrixType = Eigen::Matrix<T, Eigen::Dynamic, Eigen::Dynamic, Eigen::RowMajor>;

    std::cout << "\n=== " << label << " ===\n";
    std::cout << std::left << std::setw(14) << "Size"
              << std::right << std::setw(10) << "Time (ms)"
              << std::setw(14) << "GFLOPS" << "\n";
    std::cout << std::string(38, '-') << "\n";

    for (std::size_t n : {512, 1024, 2048, 4096}) {
        std::vector<T> a(n * n);
        std::vector<T> b(n * n);

        for (auto& v : a) v = static_cast<T>(dist(rng));
        for (auto& v : b) v = static_cast<T>(dist(rng));

        // Use Eigen directly for the benchmark
        Eigen::Map<const MatrixType> ma(a.data(), n, n);
        Eigen::Map<const MatrixType> mb(b.data(), n, n);

        auto ms = timer.benchmark([&]() {
            MatrixType mc = ma * mb;
        });

        double flops = 2.0 * n * n * n;
        double gflops = flops / (ms / 1000.0) / 1e9;

        std::cout << std::left << std::setw(14) << (std::to_string(n) + "x" + std::to_string(n))
                  << std::right << std::setw(10) << ms
                  << std::setw(12) << std::fixed << std::setprecision(2) << gflops << "\n";
    }
}

int main() {
    std::mt19937_64 rng(42);
    std::uniform_int_distribution<uint64_t> int_dist;
    std::uniform_real_distribution<double> dbl_dist(0.0, 1.0);
    std::uniform_real_distribution<float> flt_dist(0.0f, 1.0f);

    md_ml::Timer timer;

    benchSuite<uint64_t>("uint64_t (used by SPDZ2k)", int_dist, rng, timer);
    benchSuite<double>("double", dbl_dist, rng, timer);
    benchSuite<float>("float", flt_dist, rng, timer);

    return 0;
}

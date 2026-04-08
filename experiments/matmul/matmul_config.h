// Matrix multiplication experiment config

#ifndef MD_ML_MATMUL_CONFIG_H
#define MD_ML_MATMUL_CONFIG_H


#include <string>
#include <cstddef>

namespace md_ml::experiments::matmul {

const std::string kJobName = "MatMul";
constexpr std::size_t dim = 4096;  // 4096x4096 matrix multiplication

}


#endif //MD_ML_MATMUL_CONFIG_H

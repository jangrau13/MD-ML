add_executable(matmul_bench matmul_bench.cpp)
target_link_libraries(matmul_bench Eigen3::Eigen md-ml-timer)

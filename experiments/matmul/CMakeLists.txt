add_executable(matmul_party_0 matmul_party_0.cpp matmul_config.h)
add_executable(matmul_party_1 matmul_party_1.cpp matmul_config.h)
add_executable(matmul_fake_offline matmul_fake_offline.cpp matmul_config.h)

target_link_libraries(matmul_party_0 ${ONLINE_LIB})
target_link_libraries(matmul_party_1 ${ONLINE_LIB})
target_link_libraries(matmul_fake_offline ${FAKE_OFFLINE_LIB})

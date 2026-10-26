cmake_minimum_required(VERSION 3.21)
project(red4ext_rs_bridge LANGUAGES CXX)

add_library(red4ext_rs_bridge STATIC bridge.cpp)
target_compile_features(red4ext_rs_bridge PRIVATE cxx_std_20)
target_compile_definitions(red4ext_rs_bridge PRIVATE RED4EXT_STATIC_LIB WIN32_LEAN_AND_MEAN)
target_include_directories(red4ext_rs_bridge PRIVATE
    "${RED4EXT_SDK_DIR}/include"
    "${RED4EXT_SDK_DIR}/vendor/D3D12MemAlloc"
)
install(TARGETS red4ext_rs_bridge ARCHIVE DESTINATION lib)

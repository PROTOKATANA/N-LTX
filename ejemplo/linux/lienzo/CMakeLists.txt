cmake_minimum_required(VERSION 3.25)

# El pegamento entre el pintor de N-LTX y las texturas de Flutter : cada proyecto
# pone el suyo, sobre su propia base GPU. La biblioteca Rust con la que habla por
# cxx la enlaza cargokit.
add_library(ltx_lienzo STATIC
  "familia_lienzo.cc"
  "gpu_lienzo.cc"
)
apply_standard_settings(ltx_lienzo)
target_compile_features(ltx_lienzo PUBLIC cxx_std_17)
target_compile_options(ltx_lienzo PRIVATE -Wextra -Wpedantic)
target_include_directories(ltx_lienzo PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}")
target_link_libraries(ltx_lienzo PUBLIC ltx_pintor ltx_gpu)

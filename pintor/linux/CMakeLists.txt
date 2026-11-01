cmake_minimum_required(VERSION 3.25)

# Lo que genera cxx al compilar el crate `pintor`.
if(NOT NLTX_CXX)
  message(FATAL_ERROR "Falta NLTX_CXX: la carpeta adonde el build.rs de pintor copia lo que genera cxx")
endif()

add_subdirectory(base)

add_library(ltx_pintor STATIC
  "ejecucion_lienzo.cc"
  "familia_lienzo.cc"
  "gpu_lienzo.cc"
  "recursos_lienzo.cc"
  "${NLTX_CXX}/puente_lienzo.rs.cc"
  "${NLTX_CXX}/src/cxx.cc"
)
apply_standard_settings(ltx_pintor)
target_compile_features(ltx_pintor PUBLIC cxx_std_17)
target_compile_options(ltx_pintor PRIVATE -Wextra -Wpedantic)
# el código generado no es nuestro: sus avisos no deben parar la compilación
set_source_files_properties("${NLTX_CXX}/puente_lienzo.rs.cc" "${NLTX_CXX}/src/cxx.cc"
  PROPERTIES GENERATED TRUE COMPILE_OPTIONS "-w")
target_include_directories(ltx_pintor PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}")
target_include_directories(ltx_pintor SYSTEM PRIVATE "${NLTX_CXX}")
target_link_libraries(ltx_pintor PUBLIC ltx_gpu)

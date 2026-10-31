cmake_minimum_required(VERSION 3.25)

# Lo que genera cxx al compilar el crate `pintor`.
if(NOT NLTX_CXX)
  message(FATAL_ERROR "Falta NLTX_CXX: la carpeta target/<perfil>/cxx-pintor")
endif()

add_library(cnx_ltx STATIC
  "ejecucion_lienzo.cc"
  "familia_lienzo.cc"
  "familia_masiva.cc"
  "gpu_lienzo.cc"
  "gpu_masiva.cc"
  "recursos_lienzo.cc"
  "${NLTX_CXX}/puente_lienzo.rs.cc"
  "${NLTX_CXX}/puente_masiva.rs.cc"
  "${NLTX_CXX}/src/cxx.cc"
)
apply_standard_settings(cnx_ltx)
target_compile_features(cnx_ltx PUBLIC cxx_std_17)
target_compile_options(cnx_ltx PRIVATE -Wextra -Wpedantic)
# el código generado no es nuestro: sus avisos no deben parar la compilación
set_source_files_properties("${NLTX_CXX}/puente_lienzo.rs.cc" "${NLTX_CXX}/puente_masiva.rs.cc" "${NLTX_CXX}/src/cxx.cc"
  PROPERTIES GENERATED TRUE COMPILE_OPTIONS "-w")
target_include_directories(cnx_ltx PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}")
target_include_directories(cnx_ltx SYSTEM PRIVATE "${NLTX_CXX}")
target_link_libraries(cnx_ltx PUBLIC cnx_gpu)

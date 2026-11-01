cmake_minimum_required(VERSION 3.25)

# El pintor de N-LTX. Solo necesita un dispositivo Diligent : no sabe nada de
# Flutter, de GTK ni de la ventana. Quien lo integra añade antes DiligentCore y
# dice en NLTX_CXX adónde copia el build.rs de pintor lo que genera cxx.
if(NOT NLTX_CXX)
  message(FATAL_ERROR "Falta NLTX_CXX: la carpeta adonde el build.rs de pintor copia lo que genera cxx")
endif()
if(NOT TARGET Diligent-GraphicsEngineInterface OR NOT TARGET Diligent-Common)
  message(FATAL_ERROR "Falta DiligentCore: hay que añadirlo antes que el pintor")
endif()

add_library(ltx_pintor STATIC
  "ejecucion_lienzo.cc"
  "pintor_lienzo.cc"
  "recursos_lienzo.cc"
  "${NLTX_CXX}/puente_lienzo.rs.cc"
  "${NLTX_CXX}/src/cxx.cc"
)
target_compile_features(ltx_pintor PUBLIC cxx_std_17)
target_compile_options(ltx_pintor PRIVATE -Wall -Wextra -Wpedantic "$<$<NOT:$<CONFIG:Debug>>:-O3>")
target_compile_definitions(ltx_pintor PRIVATE "$<$<NOT:$<CONFIG:Debug>>:NDEBUG>")
# el código generado no es nuestro: sus avisos no deben parar la compilación
set_source_files_properties("${NLTX_CXX}/puente_lienzo.rs.cc" "${NLTX_CXX}/src/cxx.cc"
  PROPERTIES GENERATED TRUE COMPILE_OPTIONS "-w")
target_include_directories(ltx_pintor PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}")
target_include_directories(ltx_pintor SYSTEM PUBLIC "${NLTX_CXX}")
target_link_libraries(ltx_pintor PUBLIC Diligent-GraphicsEngineInterface Diligent-Common)

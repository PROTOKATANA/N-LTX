cmake_minimum_required(VERSION 3.25)

# La base GPU del pintor : la superficie que Vulkan comparte con Flutter, el
# registro de texturas, el hilo de render, el banco y DiligentCore. Necesita los
# targets `flutter`, `PkgConfig::GTK` y `PkgConfig::EPOXY` y la función
# apply_standard_settings del CMakeLists de Linux de la app.
add_library(ltx_gpu STATIC
  "banco_gpu.cc"
  "carril.cc"
  "gpu_surface.cc"
  "lienzo.cc"
  "presentacion.cc"
  "relevo.cc"
  "texturas.cc"
)

apply_standard_settings(ltx_gpu)
target_compile_features(ltx_gpu PUBLIC cxx_std_17)
# DiligentCore entra como dependencia de sistema (SYSTEM) : sus cabeceras no
# cuentan para los avisos.
target_compile_options(ltx_gpu PRIVATE -Wextra -Wpedantic)
target_include_directories(ltx_gpu PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}")

find_package(Threads REQUIRED)
target_link_libraries(ltx_gpu PUBLIC flutter PkgConfig::GTK PkgConfig::EPOXY ${CMAKE_DL_LIBS} Threads::Threads)

# DiligentCore v2.5.6, solo con Vulkan. Se descarga una vez en pintor/deps, fuera
# de build/ para sobrevivir a `flutter clean`, u otra copia con
# DILIGENT_CORE_SOURCE_DIR.
set(DILIGENT_CORE_SOURCE_DIR "$ENV{DILIGENT_CORE_SOURCE_DIR}" CACHE PATH "Copia local de DiligentCore")
set(LTX_DILIGENT_DEPS "${CMAKE_CURRENT_SOURCE_DIR}/../../deps/DiligentCore")
if(NOT DILIGENT_CORE_SOURCE_DIR AND EXISTS "${LTX_DILIGENT_DEPS}/CMakeLists.txt")
  set(DILIGENT_CORE_SOURCE_DIR "${LTX_DILIGENT_DEPS}")
endif()
if(DILIGENT_CORE_SOURCE_DIR AND NOT EXISTS "${DILIGENT_CORE_SOURCE_DIR}/CMakeLists.txt")
  message(FATAL_ERROR "DILIGENT_CORE_SOURCE_DIR no existe: ${DILIGENT_CORE_SOURCE_DIR}")
endif()

set(DILIGENT_NO_OPENGL ON CACHE BOOL "" FORCE)
set(DILIGENT_NO_VULKAN OFF CACHE BOOL "" FORCE)
set(DILIGENT_NO_WEBGPU ON CACHE BOOL "" FORCE)
set(DILIGENT_NO_DIRECT3D11 ON CACHE BOOL "" FORCE)
set(DILIGENT_NO_DIRECT3D12 ON CACHE BOOL "" FORCE)
set(DILIGENT_BUILD_TESTS OFF CACHE BOOL "" FORCE)

# DiligentCore declara versiones antiguas de CMake y usa install() de un modo que
# CMake 4 señala : son avisos de un tercero, y solo se silencian al configurarlo.
set(LTX_AVISOS_OBSOLETOS "${CMAKE_WARN_DEPRECATED}")
set(LTX_AVISOS_DESARROLLO "${CMAKE_SUPPRESS_DEVELOPER_WARNINGS}")
set(CMAKE_WARN_DEPRECATED OFF CACHE BOOL "" FORCE)
set(CMAKE_SUPPRESS_DEVELOPER_WARNINGS ON CACHE INTERNAL "" FORCE)
if(DILIGENT_CORE_SOURCE_DIR)
  add_subdirectory("${DILIGENT_CORE_SOURCE_DIR}" "${CMAKE_BINARY_DIR}/diligent" EXCLUDE_FROM_ALL SYSTEM)
  set(LTX_DILIGENT "${DILIGENT_CORE_SOURCE_DIR}")
else()
  include(FetchContent)
  FetchContent_Declare(diligent_core
    GIT_REPOSITORY https://github.com/DiligentGraphics/DiligentCore.git
    # v2.5.6, fijado al commit : una etiqueta se puede mover
    GIT_TAG b036337d68be2353c9950a85929acf796b9a6d50
    GIT_SUBMODULES_RECURSE TRUE
    SOURCE_DIR "${LTX_DILIGENT_DEPS}"
    SYSTEM)
  FetchContent_MakeAvailable(diligent_core)
  set(LTX_DILIGENT "${LTX_DILIGENT_DEPS}")
endif()
set(CMAKE_WARN_DEPRECATED "${LTX_AVISOS_OBSOLETOS}" CACHE BOOL "" FORCE)
set(CMAKE_SUPPRESS_DEVELOPER_WARNINGS "${LTX_AVISOS_DESARROLLO}" CACHE INTERNAL "" FORCE)

target_include_directories(ltx_gpu SYSTEM PUBLIC "${LTX_DILIGENT}/Common/interface")
# El glslang que empaqueta DiligentCore 2.5.6 no incluye <cstdint>.
target_compile_options(SPIRV PRIVATE -include cstdint)
find_package(Vulkan REQUIRED)
target_link_libraries(ltx_gpu PUBLIC Diligent-GraphicsEngineVk-static Vulkan::Vulkan)

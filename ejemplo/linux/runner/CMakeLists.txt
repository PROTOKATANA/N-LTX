cmake_minimum_required(VERSION 3.13)
project(runner LANGUAGES CXX)

# Define the application target. To change its name, change BINARY_NAME in the
# top-level CMakeLists.txt, not the value here, or `flutter run` will no longer
# work.
#
# Any new source files that you add to the application should be added here.
add_executable(${BINARY_NAME}
  "main.cc"
  "my_application.cc"
  "${FLUTTER_MANAGED_DIR}/generated_plugin_registrant.cc"
)

# Apply the standard set of build settings. This can be removed for applications
# that need different build settings.
apply_standard_settings(${BINARY_NAME})

# Add preprocessor definitions for the application ID.
add_definitions(-DAPPLICATION_ID="${APPLICATION_ID}")

# Add dependency libraries. Add any application-specific dependencies here.
target_link_libraries(${BINARY_NAME} PRIVATE flutter)
target_link_libraries(${BINARY_NAME} PRIVATE PkgConfig::GTK)

target_include_directories(${BINARY_NAME} PRIVATE "${CMAKE_SOURCE_DIR}")

# La base GPU, el pintor de N-LTX, que no sabe nada de Flutter, y lo que los une.
add_subdirectory("${CMAKE_SOURCE_DIR}/gpu" "${CMAKE_BINARY_DIR}/ltx_gpu")
add_subdirectory("${CMAKE_SOURCE_DIR}/../../pintor/nucleo" "${CMAKE_BINARY_DIR}/ltx_pintor")
apply_standard_settings(ltx_pintor)
add_subdirectory("${CMAKE_SOURCE_DIR}/lienzo" "${CMAKE_BINARY_DIR}/ltx_lienzo")
target_link_libraries(${BINARY_NAME} PRIVATE ltx_lienzo)
